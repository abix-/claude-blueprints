package internal

import (
	"os"
	"path/filepath"
)

func HookSessionStart(input []byte) ([]byte, error) {
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

	// Gather text files
	var files []string
	filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if ShouldProcessFile(path, info, projectPath, cfg.SkipPaths) {
			files = append(files, path)
		}
		return nil
	})

	// Discover sensitive values across all files
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

	autoMappings := cfg.MergeAutoMappings(allDiscovered)
	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	allMappings := cfg.BuildAllMappings(autoMappings)

	// Sanitize files
	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		original := string(content)
		sanitized := SanitizeText(original, allMappings)

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

	reverseMappings := cfg.ReverseMappings()
	transform := func(content string) string {
		return UnsanitizeText(content, reverseMappings)
	}

	SyncDir(projectPath, unsanitizedPath, cfg.SkipPaths, transform)

	return nil, nil
}

func SessionStartCmd() error {
	_, err := HookSessionStart(nil)
	return err
}

func SessionStopCmd() error {
	_, err := HookSessionStop(nil)
	return err
}
