package config

import (
	"encoding/json"
	"os"
	"path/filepath"
)

type Patterns struct {
	IPv4      bool     `json:"ipv4"`
	Hostnames []string `json:"hostnames"`
}

type Config struct {
	Mappings         map[string]string `json:"mappings"`
	AutoMappings     map[string]string `json:"autoMappings"`
	ExcludePaths     []string          `json:"excludePaths"`
	Patterns         Patterns          `json:"patterns"`
	UnsanitizedPath  string            `json:"unsanitizedPath"`
}

var DefaultExcludePaths = []string{".git", "node_modules", ".venv", "__pycache__"}

func SanitizerDir() string {
	return filepath.Join(os.Getenv("USERPROFILE"), ".claude", "sanitizer")
}

func SecretsPath() string {
	return filepath.Join(SanitizerDir(), "sanitizer.json")
}

func Load() (*Config, error) {
	return LoadFrom(SecretsPath())
}

func LoadFrom(path string) (*Config, error) {
	cfg := &Config{
		Mappings:        make(map[string]string),
		AutoMappings:    make(map[string]string),
		ExcludePaths:    DefaultExcludePaths,
		Patterns:        Patterns{IPv4: true},
		UnsanitizedPath: "~/.claude/unsanitized/{project}",
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil // return defaults
		}
		return nil, err
	}

	// Strip UTF-8 BOM if present
	if len(data) >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
		data = data[3:]
	}

	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, err
	}

	// Ensure maps are initialized
	if cfg.Mappings == nil {
		cfg.Mappings = make(map[string]string)
	}
	if cfg.AutoMappings == nil {
		cfg.AutoMappings = make(map[string]string)
	}

	return cfg, nil
}

// AllMappings returns combined mappings (manual takes precedence)
func (c *Config) AllMappings() map[string]string {
	all := make(map[string]string)
	for k, v := range c.AutoMappings {
		all[k] = v
	}
	for k, v := range c.Mappings {
		all[k] = v // manual overrides auto
	}
	return all
}

// ReverseMappings returns fake->real mappings
func (c *Config) ReverseMappings() map[string]string {
	reverse := make(map[string]string)
	for real, fake := range c.AllMappings() {
		reverse[fake] = real
	}
	return reverse
}

// ExpandUnsanitizedPath expands {project} placeholder
func (c *Config) ExpandUnsanitizedPath(projectName string) string {
	path := c.UnsanitizedPath
	if path == "" {
		path = "~/.claude/unsanitized/{project}"
	}
	// Expand ~
	if len(path) > 0 && path[0] == '~' {
		path = filepath.Join(os.Getenv("USERPROFILE"), path[1:])
	}
	// Expand {project}
	return filepath.Clean(replaceAll(path, "{project}", projectName))
}

func replaceAll(s, old, new string) string {
	for i := 0; i < len(s); {
		if i+len(old) <= len(s) && s[i:i+len(old)] == old {
			s = s[:i] + new + s[i+len(old):]
			i += len(new)
		} else {
			i++
		}
	}
	return s
}

// SaveAutoMappings saves autoMappings to sanitizer.json
func SaveAutoMappings(autoMappings map[string]string) error {
	return SaveAutoMappingsTo(SecretsPath(), autoMappings)
}

// SaveAutoMappingsTo saves autoMappings to specified path
func SaveAutoMappingsTo(path string, autoMappings map[string]string) error {
	// Load existing config
	data, err := os.ReadFile(path)
	if err != nil && !os.IsNotExist(err) {
		return err
	}

	// Strip BOM
	if len(data) >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
		data = data[3:]
	}

	var raw map[string]any
	if len(data) > 0 {
		if err := json.Unmarshal(data, &raw); err != nil {
			return err
		}
	} else {
		raw = make(map[string]any)
	}

	// Update autoMappings
	raw["autoMappings"] = autoMappings

	// Write back
	out, err := json.MarshalIndent(raw, "", "    ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, out, 0644)
}

// InitializeIfNeeded creates sanitizer.json with defaults if it doesn't exist
func InitializeIfNeeded() error {
	path := SecretsPath()
	if _, err := os.Stat(path); err == nil {
		return nil // exists
	}

	// Create directories
	os.MkdirAll(SanitizerDir(), 0755)
	os.MkdirAll(filepath.Join(os.Getenv("USERPROFILE"), ".claude", "unsanitized"), 0755)

	// Create default config
	cfg := map[string]any{
		"mappings":        map[string]string{},
		"autoMappings":    map[string]string{},
		"patterns":        map[string]any{"ipv4": true, "hostnames": []string{}},
		"unsanitizedPath": "~/.claude/unsanitized/{project}",
		"excludePaths":    DefaultExcludePaths,
	}

	data, err := json.MarshalIndent(cfg, "", "    ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}
