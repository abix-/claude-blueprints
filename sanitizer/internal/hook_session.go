// hook_session.go - Session lifecycle hooks.
// SessionStart: Sanitizes entire project before Claude sees any files.
// SessionStop: Syncs sanitized working tree back to unsanitized directory.
package internal

import (
	"os"
	"path/filepath"
)

// HookSessionStart runs when Claude Code session begins.
// Walks all project files, discovers sensitive values, and sanitizes in-place.
// This is the "bulk sanitization" that happens before Claude sees any files.
func HookSessionStart(input []byte) ([]byte, error) {
	// Create config file with examples if it doesn't exist
	if err := InitializeConfigIfNeeded(); err != nil {
		return nil, err
	}

	cfg, err := LoadConfig()
	if err != nil {
		return nil, err
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	// Phase 1: Collect all processable files
	var files []string
	filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip errors, continue walking
		}
		if ShouldProcessFile(path, info, projectPath, cfg.SkipPaths) {
			files = append(files, path)
		}
		return nil
	})

	// Phase 2: Discover all sensitive values across all files.
	// Do this in a separate pass so we have complete mappings before sanitizing.
	// Otherwise, same IP in two files could get different sanitized values.
	allDiscovered := make(map[string]string)
	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		for k, v := range DiscoverSensitiveValues(string(content), cfg) {
			if _, exists := allDiscovered[k]; !exists {
				allDiscovered[k] = v
			}
		}
	}

	// Merge discovered with existing auto mappings and save
	autoMappings := cfg.MergeAutoMappings(allDiscovered)
	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	// Build complete mapping set (auto + manual)
	allMappings := cfg.BuildAllMappings(autoMappings)

	// Phase 3: Sanitize all files with complete mappings
	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		original := string(content)
		sanitized := SanitizeText(original, allMappings)

		// Only write if content changed
		if sanitized != original {
			info, _ := os.Stat(path)
			mode := os.FileMode(0644)
			if info != nil {
				mode = info.Mode()
			}
			os.WriteFile(path, []byte(sanitized), mode)
		}
	}

	return nil, nil
}

// HookSessionStop runs when Claude Code session ends.
// Syncs the sanitized working tree to the unsanitized directory,
// reversing the sanitization so files have real values for deployment.
func HookSessionStop(input []byte) ([]byte, error) {
	cfg, err := LoadConfig()
	if err != nil {
		return nil, err
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)

	if err := os.MkdirAll(unsanitizedPath, 0755); err != nil {
		return nil, err
	}

	// Reverse mappings: sanitized -> original
	reverseMappings := cfg.ReverseMappings()

	// Transform function that unsanitizes content
	transform := func(content string) string {
		return UnsanitizeText(content, reverseMappings)
	}

	// Sync entire project to unsanitized directory with transformation
	SyncDir(projectPath, unsanitizedPath, cfg.SkipPaths, transform)

	return nil, nil
}

// SessionStartCmd is the CLI entry point for hook-session-start.
// Wraps HookSessionStart for use as a command (not hook callback).
func SessionStartCmd() error {
	_, err := HookSessionStart(nil)
	return err
}

// SessionStopCmd is the CLI entry point for hook-session-stop.
func SessionStopCmd() error {
	_, err := HookSessionStop(nil)
	return err
}
