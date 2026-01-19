package internal

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

func Exec(command string) error {
	cfg, err := LoadConfig()
	if err != nil {
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

	// Sync sanitized -> unsanitized (reverse mappings)
	reverseMappings := cfg.ReverseMappings()
	transform := func(content string) string {
		return UnsanitizeText(content, reverseMappings)
	}
	_ = SyncDir(projectPath, unsanitizedPath, cfg.SkipPaths, transform)

	// Unsanitize command string so it uses real values
	unsanitizedCmd := UnsanitizeText(command, reverseMappings)

	// Execute command in unsanitized directory via PowerShell
	cmd := exec.Command("powershell.exe", "-NoProfile", "-Command", unsanitizedCmd)
	cmd.Dir = unsanitizedPath

	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err = cmd.Run()

	// Combine output
	output := stdout.String() + stderr.String()

	// Sanitize output
	allMappings := cfg.AllMappings()
	sanitized := sanitizeTextWithFallback(output, allMappings)

	fmt.Print(sanitized)

	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		return err
	}

	return nil
}
