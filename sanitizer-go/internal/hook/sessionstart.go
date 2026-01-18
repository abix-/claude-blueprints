package hook

import (
	"os"
	"path/filepath"
	"regexp"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal/config"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/fileutil"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/sanitize"
)

// SessionStart sanitizes project files at session start
func SessionStart(input []byte) ([]byte, error) {
	// Initialize config if needed
	if err := config.InitializeIfNeeded(); err != nil {
		return nil, err
	}

	cfg, err := config.Load()
	if err != nil {
		return nil, err
	}

	projectPath, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	// Copy existing autoMappings
	autoMappings := make(map[string]string)
	for k, v := range cfg.AutoMappings {
		autoMappings[k] = v
	}

	// Gather text files
	var files []string
	filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}

		relPath, _ := filepath.Rel(projectPath, path)
		if fileutil.IsExcludedPath(relPath, cfg.ExcludePaths) {
			return nil
		}
		if info.Size() == 0 || info.Size() > 10*1024*1024 {
			return nil
		}
		if fileutil.IsBinary(path) {
			return nil
		}

		files = append(files, path)
		return nil
	})

	// Discover IPs and hostnames
	discovered := make(map[string]string) // value -> type (ip/hostname)
	ipRegex := sanitize.IPv4Regex()

	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		text := string(content)

		// Find IPs
		if cfg.Patterns.IPv4 {
			for _, ip := range ipRegex.FindAllString(text, -1) {
				if !sanitize.IsExcludedIP(ip) {
					discovered[ip] = "ip"
				}
			}
		}

		// Find hostnames
		for _, pattern := range cfg.Patterns.Hostnames {
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
	for real, typ := range discovered {
		if _, exists := cfg.Mappings[real]; exists {
			continue
		}
		if _, exists := autoMappings[real]; exists {
			continue
		}
		if typ == "ip" {
			autoMappings[real] = sanitize.NewFakeIP()
		} else {
			autoMappings[real] = sanitize.NewFakeHostname()
		}
	}

	// Save if new mappings discovered
	if len(autoMappings) > len(cfg.AutoMappings) {
		config.SaveAutoMappings(autoMappings)
	}

	// Build combined mappings (manual takes precedence)
	allMappings := make(map[string]string)
	for k, v := range autoMappings {
		allMappings[k] = v
	}
	for k, v := range cfg.Mappings {
		allMappings[k] = v
	}

	// Sanitize files
	for _, path := range files {
		content, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		original := string(content)
		sanitized := sanitize.Text(original, allMappings)

		if sanitized != original {
			// Preserve file mode
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

// SessionStartCmd is for CLI invocation (not hook JSON)
func SessionStartCmd() error {
	_, err := SessionStart(nil)
	return err
}
