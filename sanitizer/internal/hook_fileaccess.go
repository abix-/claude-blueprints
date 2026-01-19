package internal

import (
	"encoding/json"
	"log"
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
		ToolName      string `json:"tool_name"`
		ToolInput     struct {
			FilePath string `json:"file_path"`
			Content  string `json:"content"`
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

	// For Write tool: sanitize content before writing
	if hookData.ToolName == "Write" && hookData.ToolInput.Content != "" {
		return sanitizeWriteContent(hookData.ToolInput.FilePath, hookData.ToolInput.Content)
	}

	// Sanitize file on read/edit if needed
	SanitizeSingleFile(hookData.ToolInput.FilePath)

	return nil, nil
}

func sanitizeWriteContent(filePath, content string) ([]byte, error) {
	cfg, err := LoadConfig()
	if err != nil {
		return nil, nil
	}

	// Discover any new sensitive values in content being written
	discovered := DiscoverSensitiveValues(content, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	allMappings := cfg.BuildAllMappings(autoMappings)
	sanitized := SanitizeText(content, allMappings)

	if sanitized == content {
		return nil, nil
	}

	// Return updated content
	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName":      "PreToolUse",
			"permissionDecision": "allow",
			"updatedInput": map[string]any{
				"file_path": filePath,
				"content":   sanitized,
			},
		},
	})
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

	sanitized := SanitizeText(currentContent, cfg.BuildAllMappings(autoMappings))
	if sanitized == currentContent {
		return
	}

	if err := os.WriteFile(filePath, []byte(sanitized), info.Mode()); err != nil {
		log.Printf("sanitizer: failed to write %s: %v", filePath, err)
	}
	if err := os.MkdirAll(filepath.Dir(unsanitizedFilePath), 0755); err != nil {
		log.Printf("sanitizer: failed to create dir %s: %v", filepath.Dir(unsanitizedFilePath), err)
	}
	if err := os.WriteFile(unsanitizedFilePath, content, info.Mode()); err != nil {
		log.Printf("sanitizer: failed to write backup %s: %v", unsanitizedFilePath, err)
	}
}
