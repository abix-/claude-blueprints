// hook_fileaccess.go - PreToolUse hook for Read, Edit, Write tools.
// Blocks access to sanitizer internals and ensures files are sanitized before Claude sees them.
package internal

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// Paths that should never be accessible to Claude
var blockedPathPatterns = []*regexp.Regexp{
	regexp.MustCompile(`\.claude[/\\]sanitizer[/\\]sanitizer\.json$`), // Config with real values
	regexp.MustCompile(`\.claude[/\\]unsanitized[/\\]`),              // Unsanitized file copies
}

// HookFileAccess processes Read/Edit/Write tool invocations.
// - Blocks access to sensitive paths
// - Sanitizes file content on read/edit (in-place modification)
// - Sanitizes content before write (modifies tool input)
func HookFileAccess(input []byte) ([]byte, error) {
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolName      string `json:"tool_name"`
		ToolInput     struct {
			FilePath string `json:"file_path"`
			Content  string `json:"content"` // Only for Write tool
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

	// Normalize path separators for consistent matching
	path := strings.ReplaceAll(hookData.ToolInput.FilePath, "\\", "/")

	// Block access to sanitizer internals
	for _, pattern := range blockedPathPatterns {
		if pattern.MatchString(path) {
			return DenyResponse("Access blocked: sensitive sanitizer file")
		}
	}

	// Write tool: sanitize content BEFORE it's written to disk
	if hookData.ToolName == "Write" && hookData.ToolInput.Content != "" {
		return sanitizeWriteContent(hookData.ToolInput.FilePath, hookData.ToolInput.Content)
	}

	// Read/Edit: sanitize file on disk before Claude reads it
	SanitizeSingleFile(hookData.ToolInput.FilePath)

	return nil, nil
}

// sanitizeWriteContent modifies Write tool input to sanitize content before writing.
// Returns updated tool input with sanitized content, or nil if no changes needed.
func sanitizeWriteContent(filePath, content string) ([]byte, error) {
	cfg, err := LoadConfig()
	if err != nil {
		return nil, nil
	}

	// Discover any new sensitive values in the content Claude is writing
	discovered := DiscoverSensitiveValues(content, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	allMappings := cfg.BuildAllMappings(autoMappings)
	sanitized := SanitizeText(content, allMappings)

	// No changes needed
	if sanitized == content {
		return nil, nil
	}

	// Return modified tool input with sanitized content
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

// SanitizeSingleFile sanitizes a file in-place before Claude reads it.
// Called on every Read/Edit to catch files that weren't sanitized at session start
// (new files, modified files, files outside initial walk).
//
// Idempotent: if content is already sanitized, file is unchanged.
// Also saves original content to unsanitized directory for later restoration.
func SanitizeSingleFile(filePath string) {
	projectPath, err := os.Getwd()
	if err != nil {
		return
	}

	filePath = filepath.Clean(filePath)
	projectPath = filepath.Clean(projectPath)

	// Only process files within the project directory
	// Case-insensitive comparison for Windows (C:\Foo vs c:\foo)
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

	// Calculate paths for unsanitized backup
	relPath, _ := filepath.Rel(projectPath, filePath)
	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)
	unsanitizedFilePath := filepath.Join(unsanitizedPath, relPath)

	content, err := os.ReadFile(filePath)
	if err != nil {
		return
	}
	currentContent := string(content)

	// Discover new sensitive values and merge with existing
	discovered := DiscoverSensitiveValues(currentContent, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	sanitized := SanitizeText(currentContent, cfg.BuildAllMappings(autoMappings))

	// Already sanitized (or no sensitive values) - nothing to do
	if sanitized == currentContent {
		return
	}

	// Write sanitized content to working tree (Claude sees this)
	if err := os.WriteFile(filePath, []byte(sanitized), info.Mode()); err != nil {
		log.Printf("sanitizer: failed to write %s: %v", filePath, err)
	}

	// Save original (unsanitized) content for later restoration
	if err := os.MkdirAll(filepath.Dir(unsanitizedFilePath), 0755); err != nil {
		log.Printf("sanitizer: failed to create dir %s: %v", filepath.Dir(unsanitizedFilePath), err)
	}
	if err := os.WriteFile(unsanitizedFilePath, content, info.Mode()); err != nil {
		log.Printf("sanitizer: failed to write backup %s: %v", unsanitizedFilePath, err)
	}
}
