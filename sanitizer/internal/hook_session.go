package internal

import (
	"os"
	"path/filepath"
	"regexp"
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

	autoMappings := make(map[string]string)
	for k, v := range cfg.MappingsAuto {
		autoMappings[k] = v
	}

	// Gather text files
	var files []string
	filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}

		relPath, _ := filepath.Rel(projectPath, path)
		if IsSkippedPath(relPath, cfg.SkipPaths) {
			return nil
		}
		if info.Size() == 0 || info.Size() > 10*1024*1024 {
			return nil
		}
		if IsBinary(path) {
			return nil
		}

		files = append(files, path)
		return nil
	})

	// Discover IPs and hostnames
	discovered := make(map[string]string)
	ipRegex := IPv4Regex()

	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		text := string(content)

		for _, ip := range ipRegex.FindAllString(text, -1) {
			if !IsExcludedIP(ip) {
				discovered[ip] = "ip"
			}
		}

		for _, pattern := range cfg.HostnamePatterns {
			re, err := regexp.Compile(`(?i)[a-zA-Z0-9][-a-zA-Z0-9\.]*` + regexp.QuoteMeta(pattern))
			if err != nil {
				continue
			}
			for _, match := range re.FindAllString(text, -1) {
				discovered[match] = "hostname"
			}
		}
	}

	// Generate mappings for new discoveries
	for value, typ := range discovered {
		if _, exists := cfg.MappingsManual[value]; exists {
			continue
		}
		if _, exists := autoMappings[value]; exists {
			continue
		}
		if typ == "ip" {
			autoMappings[value] = NewSanitizedIP()
		} else {
			autoMappings[value] = NewSanitizedHostname()
		}
	}

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	// Build combined mappings
	allMappings := make(map[string]string)
	for k, v := range autoMappings {
		allMappings[k] = v
	}
	for k, v := range cfg.MappingsManual {
		allMappings[k] = v
	}

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
