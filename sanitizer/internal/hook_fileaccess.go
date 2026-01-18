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

// SanitizeSingleFile sanitizes a file on every read/edit.
// Idempotent: if content is already sanitized, no changes made.
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
	if IsSkippedPath(relPath, cfg.SkipPaths) {
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

	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)
	unsanitizedFilePath := filepath.Join(unsanitizedPath, relPath)

	// Read current file content
	content, err := os.ReadFile(filePath)
	if err != nil {
		return
	}
	currentContent := string(content)

	// Discover IPs/hostnames in current content
	autoMappings := make(map[string]string)
	for k, v := range cfg.MappingsAuto {
		autoMappings[k] = v
	}

	ipRegex := IPv4Regex()
	for _, ip := range ipRegex.FindAllString(currentContent, -1) {
		if !IsExcludedIP(ip) {
			if _, exists := cfg.MappingsManual[ip]; !exists {
				if _, exists := autoMappings[ip]; !exists {
					autoMappings[ip] = NewSanitizedIP()
				}
			}
		}
	}

	for _, pattern := range cfg.HostnamePatterns {
		re, err := regexp.Compile(`(?i)[a-zA-Z0-9][-a-zA-Z0-9\.]*` + pattern)
		if err != nil {
			continue
		}
		for _, match := range re.FindAllString(currentContent, -1) {
			if _, exists := cfg.MappingsManual[match]; !exists {
				if _, exists := autoMappings[match]; !exists {
					autoMappings[match] = NewSanitizedHostname()
				}
			}
		}
	}

	// Save autoMappings if changed
	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	// Build all mappings
	allMappings := make(map[string]string)
	for k, v := range autoMappings {
		allMappings[k] = v
	}
	for k, v := range cfg.MappingsManual {
		allMappings[k] = v
	}

	// Sanitize content
	sanitized := SanitizeText(currentContent, allMappings)

	// If unchanged, already clean - nothing to do
	if sanitized == currentContent {
		return
	}

	// Write sanitized to working tree
	os.WriteFile(filePath, []byte(sanitized), info.Mode())

	// Write unsanitized content to unsanitized dir (update backup)
	os.MkdirAll(filepath.Dir(unsanitizedFilePath), 0755)
	os.WriteFile(unsanitizedFilePath, content, info.Mode())
}
