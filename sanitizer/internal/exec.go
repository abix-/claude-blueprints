// exec.go - Execute commands with real (unsanitized) values.
// Used when Claude runs PowerShell commands that need to interact with real infrastructure.
// Flow: sync to unsanitized dir -> run command with real values -> sanitize output
package internal

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

// Exec runs a command in the unsanitized directory with real values.
// Called via: sanitizer.exe exec '<command>'
//
// Steps:
// 1. Sync working tree to unsanitized directory (reversing sanitization)
// 2. Unsanitize the command string itself (replace fake IPs with real ones)
// 3. Execute command in unsanitized directory
// 4. Sanitize output before printing (so Claude sees sanitized values)
// 5. Preserve exit code (so command failures propagate correctly)
func Exec(command string) error {
	cfg, err := LoadConfig()
	if err != nil {
		// fmt.Errorf with %w wraps the error, preserving the original.
		// Caller can use errors.Unwrap() to get the underlying error.
		return fmt.Errorf("config load: %w", err)
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("getwd: %w", err)
	}

	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)

	if err := os.MkdirAll(unsanitizedPath, 0755); err != nil {
		return fmt.Errorf("mkdir %s: %w", unsanitizedPath, err)
	}

	// Sync working tree (sanitized) -> unsanitized directory.
	// Transform reverses sanitization: fake values -> real values.
	reverseMappings := cfg.ReverseMappings()
	transform := func(content string) string {
		return UnsanitizeText(content, reverseMappings)
	}
	_ = SyncDir(projectPath, unsanitizedPath, cfg.SkipPaths, transform)

	// The command Claude wrote uses sanitized values (e.g., 111.x.x.x).
	// Unsanitize it so it references real infrastructure.
	unsanitizedCmd := UnsanitizeText(command, reverseMappings)

	// exec.Command creates a command but doesn't run it yet.
	// -NoProfile skips loading PowerShell profile for faster startup.
	cmd := exec.Command("powershell.exe", "-NoProfile", "-Command", unsanitizedCmd)
	cmd.Dir = unsanitizedPath // Run in unsanitized directory

	// Capture stdout and stderr separately into buffers.
	// bytes.Buffer implements io.Writer, so we can assign it directly.
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	// Run() blocks until command completes
	err = cmd.Run()

	// Combine output (note: loses interleaving order between stdout/stderr)
	output := stdout.String() + stderr.String()

	// Sanitize output so Claude doesn't see real values.
	// Uses fallback to catch any IPs not in mappings.
	allMappings := cfg.AllMappings()
	sanitized := sanitizeTextWithFallback(output, allMappings)

	// Print to stdout (goes back to Claude via bash tool)
	fmt.Print(sanitized)

	// Preserve the command's exit code so failures propagate correctly.
	// Type assertion: err.(*exec.ExitError) checks if err is an ExitError.
	// The "comma ok" idiom returns (value, bool) - true if assertion succeeded.
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		return err
	}

	return nil
}
