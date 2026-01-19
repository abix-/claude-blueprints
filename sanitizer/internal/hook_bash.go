// hook_bash.go - PreToolUse hook for Bash commands.
// Routes commands into three categories:
// - BLOCK: Commands accessing sanitizer config or unsanitized directory (denied)
// - UNSANITIZED: PowerShell commands (wrapped to run in unsanitized directory)
// - SANITIZED: Everything else (runs as-is in working tree with sanitized values)
package internal

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// Commands that need real values - run UNSANITIZED
// PowerShell scripts typically interact with real infrastructure
var unsanitizedCmdPatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)^\s*powershell`), // powershell.exe
	regexp.MustCompile(`(?i)^\s*pwsh`),       // pwsh (PS Core)
	regexp.MustCompile(`(?i)\.ps1(\s|$|")`),  // *.ps1 scripts
	regexp.MustCompile(`^\s*&\s`),            // & (call operator)
}

// HookBash processes Bash tool invocations before execution.
// Claude Code sends hook input as JSON on stdin.
// Returns JSON response or nil (nil = allow command as-is).
func HookBash(input []byte) ([]byte, error) {
	// Anonymous struct - declare and use inline, no need for separate type.
	// Only parse the fields we need from the hook JSON.
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolInput     struct {
			Command string `json:"command"`
		} `json:"tool_input"`
	}

	if err := json.Unmarshal(input, &hookData); err != nil {
		return nil, nil // Parse error = allow (fail open)
	}

	// Only handle PreToolUse events
	if hookData.HookEventName != "PreToolUse" {
		return nil, nil
	}

	command := hookData.ToolInput.Command
	if command == "" {
		return nil, nil
	}

	// Load config for blocked paths
	cfg, err := LoadConfig()
	if err != nil {
		return nil, nil // Config error = allow (fail open)
	}

	// Normalize path separators for consistent matching
	normalizedCmd := strings.ReplaceAll(command, "\\", "/")

	// BLOCK: Deny access to sanitizer internals (paths from config)
	for _, pattern := range cfg.BlockedPathRegexes() {
		if pattern.MatchString(normalizedCmd) {
			return DenyResponse("Blocked")
		}
	}

	// Check if command should run UNSANITIZED (with real values)
	isUnsanitized := false
	for _, pattern := range unsanitizedCmdPatterns {
		if pattern.MatchString(command) {
			isUnsanitized = true
			break
		}
	}

	// SANITIZED (default): Let command run as-is in working tree
	if !isUnsanitized {
		return nil, nil
	}

	// UNSANITIZED: Wrap command to run through sanitizer exec.
	// This syncs to unsanitized directory, runs command with real values,
	// then sanitizes the output before returning to Claude.
	sanitizerExe := filepath.Join(os.Getenv("USERPROFILE"), ".claude", "bin", "sanitizer.exe")

	// Escape single quotes for bash: ' becomes '\'' (end quote, escaped quote, start quote)
	escapedCmd := strings.ReplaceAll(command, "'", `'\''`)

	wrappedCommand := fmt.Sprintf(`'%s' exec '%s'`, sanitizerExe, escapedCmd)

	return allowWithUpdatedCommand(wrappedCommand)
}

// DenyResponse returns JSON that tells Claude Code to block the tool call.
func DenyResponse(reason string) ([]byte, error) {
	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName":      "PreToolUse",
			"permissionDecision": "deny",
			"reason":             reason,
		},
	})
}

// allowWithUpdatedCommand returns JSON that allows the tool but modifies the command.
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
