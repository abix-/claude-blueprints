package hook

import (
	"os"
	"path/filepath"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal/config"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/fileutil"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/sanitize"
)

// SessionStop syncs project to unsanitized directory at session end
func SessionStop(input []byte) ([]byte, error) {
	cfg, err := config.Load()
	if err != nil {
		return nil, err
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	projectName := filepath.Base(projectPath)
	unsanitizedPath := cfg.ExpandUnsanitizedPath(projectName)

	// Create destination directory
	if err := os.MkdirAll(unsanitizedPath, 0755); err != nil {
		return nil, err
	}

	// Sync with unsanitization
	reverseMappings := cfg.ReverseMappings()
	transform := func(content string) string {
		return sanitize.Unsanitize(content, reverseMappings)
	}

	fileutil.SyncDir(projectPath, unsanitizedPath, cfg.ExcludePaths, transform)

	return nil, nil
}

// SessionStopCmd is for CLI invocation (not hook JSON)
func SessionStopCmd() error {
	_, err := SessionStop(nil)
	return err
}
