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
			return DenyResponse("Access blocked: sensitive sanitizer file")
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

	filePath = filepath.Clean(filePath)
	projectPath = filepath.Clean(projectPath)

	// Check if file is under project (case-insensitive for Windows)
	if !strings.HasPrefix(strings.ToLower(filePath), strings.ToLower(projectPath)) {
		return
	}

	cfg, err := LoadConfig()
	if err != nil {
		return
	}

	info, err := os.Stat(filePath)
	if err != nil {
		return
	}

	if !ShouldProcessFile(filePath, info, projectPath, cfg.SkipPaths) {
		return
	}

	relPath, _ := filepath.Rel(projectPath, filePath)
	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)
	unsanitizedFilePath := filepath.Join(unsanitizedPath, relPath)

	content, err := os.ReadFile(filePath)
	if err != nil {
		return
	}
	currentContent := string(content)

	// Discover and merge auto mappings
	discovered := DiscoverSensitiveValues(currentContent, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	// Build all mappings (auto + manual, manual wins)
	allMappings := make(map[string]string)
	for k, v := range autoMappings {
		allMappings[k] = v
	}
	for k, v := range cfg.MappingsManual {
		allMappings[k] = v
	}

	sanitized := SanitizeText(currentContent, allMappings)
	if sanitized == currentContent {
		return
	}

	os.WriteFile(filePath, []byte(sanitized), info.Mode())
	os.MkdirAll(filepath.Dir(unsanitizedFilePath), 0755)
	os.WriteFile(unsanitizedFilePath, content, info.Mode())
}
