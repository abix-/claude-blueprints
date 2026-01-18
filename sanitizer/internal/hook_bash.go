package internal

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

var (
	blockedCmdPatterns = []*regexp.Regexp{
		regexp.MustCompile(`[/\\]sanitizer\.json($|[\s"'])`),
		regexp.MustCompile(`\.claude[/\\]unsanitized`),
	}

	realCmdPatterns = []*regexp.Regexp{
		regexp.MustCompile(`(?i)^\s*powershell`),
		regexp.MustCompile(`(?i)^\s*pwsh`),
		regexp.MustCompile(`(?i)\.ps1(\s|$|")`),
		regexp.MustCompile(`^\s*&\s`),
	}
)

func HookBash(input []byte) ([]byte, error) {
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolInput     struct {
			Command string `json:"command"`
		} `json:"tool_input"`
	}

	if err := json.Unmarshal(input, &hookData); err != nil {
		return nil, nil
	}

	if hookData.HookEventName != "PreToolUse" {
		return nil, nil
	}

	command := hookData.ToolInput.Command
	if command == "" {
		return nil, nil
	}

	// DENY
	for _, pattern := range blockedCmdPatterns {
		if pattern.MatchString(command) {
			return denyResponse("Blocked")
		}
	}

	// Check if REAL
	isReal := false
	for _, pattern := range realCmdPatterns {
		if pattern.MatchString(command) {
			isReal = true
			break
		}
	}

	// FAKE (default)
	if !isReal {
		return nil, nil
	}

	// REAL - wrap with sanitizer exec
	sanitizerExe := filepath.Join(os.Getenv("USERPROFILE"), ".claude", "bin", "sanitizer.exe")

	// Escape single quotes for bash: ' → '\''
	escapedCmd := strings.ReplaceAll(command, "'", `'\''`)

	wrappedCommand := fmt.Sprintf(`'%s' exec '%s'`, sanitizerExe, escapedCmd)

	return allowWithUpdatedCommand(wrappedCommand)
}

func denyResponse(reason string) ([]byte, error) {
	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName":      "PreToolUse",
			"permissionDecision": "deny",
			"reason":             reason,
		},
	})
}

func allowWithUpdatedCommand(command string) ([]byte, error) {
	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName":      "PreToolUse",
			"permissionDecision": "allow",
			"updatedInput": map[string]any{
				"command": command,
			},
		},
	})
}
