package internal

import (
	"encoding/json"
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

	return nil, nil
}
