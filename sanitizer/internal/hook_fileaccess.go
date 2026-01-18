package internal

import (
	"encoding/json"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

var blockedPathPatterns = []*regexp.Regexp{
	regexp.MustCompile(`\.claude[/\\]sanitizer[/\\]sanitizer\.json$`),
	regexp.MustCompile(`\.claude[/\\]unsanitized[/\\]`),
}

func HookFileAccess(input []byte) ([]byte, error) {
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolInput     struct {
			FilePath string `json:"file_path"`
		} `json:"tool_input"`
	}

	if err := json.Unmarshal(input, &hookData); err != nil {
		return nil, nil
	}

	if hookData.HookEventName != "PreToolUse" {
		return nil, nil
	}

	if hookData.ToolInput.FilePath == "" {
		return nil, nil
	}

	path := strings.ReplaceAll(hookData.ToolInput.FilePath, "\\", "/")

	for _, pattern := range blockedPathPatterns {
		if pattern.MatchString(path) {
			return json.Marshal(map[string]any{
				"hookSpecificOutput": map[string]any{
					"hookEventName":      "PreToolUse",
					"permissionDecision": "deny",
					"reason":             "Access blocked: sensitive sanitizer file",
				},
			})
		}
	}

	// Sanitize file on read if needed
	SanitizeSingleFile(hookData.ToolInput.FilePath)

	return nil, nil
}

// SanitizeSingleFile sanitizes a file if it's new/changed since session start.
// Writes sanitized version to working tree, real version to unsanitized dir.
func SanitizeSingleFile(filePath string) {
	projectPath, err := os.Getwd()
	if err != nil {
		return
	}

	// Normalize paths
	filePath = filepath.Clean(filePath)
	projectPath = filepath.Clean(projectPath)

	// Check if file is under project
	if !strings.HasPrefix(strings.ToLower(filePath), strings.ToLower(projectPath)) {
		return
	}

	relPath, err := filepath.Rel(projectPath, filePath)
	if err != nil || strings.HasPrefix(relPath, "..") {
		return
	}

	cfg, err := LoadConfig()
	if err != nil {
		return
	}

	// Check exclusions
	if IsExcludedPath(relPath, cfg.ExcludePaths) {
		return
	}

	// Check file exists and size
	info, err := os.Stat(filePath)
	if err != nil || info.IsDir() || info.Size() == 0 || info.Size() > 10*1024*1024 {
		return
	}

	if IsBinary(filePath) {
		return
	}

	// Check if already sanitized (exists in unsanitized dir)
	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)
	unsanitizedFilePath := filepath.Join(unsanitizedPath, relPath)

	if _, err := os.Stat(unsanitizedFilePath); err == nil {
		// File exists in unsanitized dir - already processed
		return
	}

	// Read file content (real values)
	content, err := os.ReadFile(filePath)
	if err != nil {
		return
	}
	realContent := string(content)

	// Discover IPs/hostnames
	autoMappings := make(map[string]string)
	for k, v := range cfg.AutoMappings {
		autoMappings[k] = v
	}

	ipRegex := IPv4Regex()
	if cfg.Patterns.IPv4 {
		for _, ip := range ipRegex.FindAllString(realContent, -1) {
			if !IsExcludedIP(ip) {
				if _, exists := cfg.Mappings[ip]; !exists {
					if _, exists := autoMappings[ip]; !exists {
						autoMappings[ip] = NewFakeIP()
					}
				}
			}
		}
	}

	for _, pattern := range cfg.Patterns.Hostnames {
		re, err := regexp.Compile(`(?i)[a-zA-Z0-9][-a-zA-Z0-9\.]*` + pattern)
		if err != nil {
			continue
		}
		for _, match := range re.FindAllString(realContent, -1) {
			if _, exists := cfg.Mappings[match]; !exists {
				if _, exists := autoMappings[match]; !exists {
					autoMappings[match] = NewFakeHostname()
				}
			}
		}
	}

	// Save autoMappings if changed
	if len(autoMappings) > len(cfg.AutoMappings) {
		SaveAutoMappings(autoMappings)
	}

	// Build all mappings
	allMappings := make(map[string]string)
	for k, v := range autoMappings {
		allMappings[k] = v
	}
	for k, v := range cfg.Mappings {
		allMappings[k] = v
	}

	// Sanitize content
	sanitized := SanitizeText(realContent, allMappings)

	// Write sanitized to working tree
	if sanitized != realContent {
		os.WriteFile(filePath, []byte(sanitized), info.Mode())
	}

	// Write real content to unsanitized dir
	os.MkdirAll(filepath.Dir(unsanitizedFilePath), 0755)
	os.WriteFile(unsanitizedFilePath, content, info.Mode())
}
