package hook

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal/config"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/fileutil"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/sanitize"
)

var (
	// Commands that should be blocked entirely
	blockedPatterns = []*regexp.Regexp{
		regexp.MustCompile(`[/\\]sanitizer\.json($|[\s"'])`),
		regexp.MustCompile(`\.claude[/\\]unsanitized`),
	}

	// Commands that need to run in unsanitized directory with real values
	realPatterns = []*regexp.Regexp{
		regexp.MustCompile(`(?i)^\s*powershell`),
		regexp.MustCompile(`(?i)^\s*pwsh`),
		regexp.MustCompile(`(?i)\.ps1(\s|$|")`),
		regexp.MustCompile(`^\s*&\s`),
		regexp.MustCompile(`(?i)^\s*ansible\b`),
		regexp.MustCompile(`(?i)^\s*awx\b`),
	}
)

func Bash(input []byte) ([]byte, error) {
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolInput     struct {
			Command string `json:"command"`
		} `json:"tool_input"`
	}

	if err := json.Unmarshal(input, &hookData); err != nil {
		return nil, nil // invalid input, allow
	}

	if hookData.HookEventName != "PreToolUse" {
		return nil, nil
	}

	command := hookData.ToolInput.Command
	if command == "" {
		return nil, nil
	}

	// === DENY ===
	for _, pattern := range blockedPatterns {
		if pattern.MatchString(command) {
			return denyResponse("Blocked")
		}
	}

	// === Check if REAL ===
	isReal := false
	for _, pattern := range realPatterns {
		if pattern.MatchString(command) {
			isReal = true
			break
		}
	}

	// === FAKE (default) - allow command as-is ===
	if !isReal {
		return nil, nil
	}

	// === REAL - execute in unsanitized directory ===
	cfg, err := config.Load()
	if err != nil {
		return nil, fmt.Errorf("config load: %w", err)
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return nil, fmt.Errorf("getwd: %w", err)
	}
	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)

	// Create unsanitized directory
	if err := os.MkdirAll(unsanitizedPath, 0755); err != nil {
		return nil, fmt.Errorf("mkdir %s: %w", unsanitizedPath, err)
	}

	// Sync files to unsanitized directory (unsanitizing content)
	reverseMappings := cfg.ReverseMappings()
	transform := func(content string) string {
		return sanitize.Unsanitize(content, reverseMappings)
	}
	_ = fileutil.SyncDir(projectPath, unsanitizedPath, cfg.ExcludePaths, transform)

	// Build wrapped command that:
	// 1. Runs in unsanitized directory
	// 2. Sanitizes output
	sanitizerExe := filepath.Join(os.Getenv("USERPROFILE"), ".claude", "bin", "sanitizer.exe")
	escapedCommand := strings.ReplaceAll(command, `"`, `\"`)

	wrappedCommand := fmt.Sprintf(
		`powershell.exe -NoProfile -Command "Set-Location '%s'; $o = cmd /c \"%s\" 2>&1 | Out-String; $o | & '%s' sanitize-ips"`,
		unsanitizedPath,
		escapedCommand,
		sanitizerExe,
	)

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
