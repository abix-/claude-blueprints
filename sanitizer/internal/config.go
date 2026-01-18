package internal

import (
	"encoding/json"
	"os"
	"path/filepath"
)

type Config struct {
	MappingsManual   map[string]string `json:"mappingsManual"`
	MappingsAuto     map[string]string `json:"mappingsAuto"`
	SkipPaths        []string          `json:"skipPaths"`
	HostnamePatterns []string          `json:"hostnamePatterns"`
	UnsanitizedPath  string            `json:"unsanitizedPath"`
}

var DefaultSkipPaths = []string{".git", "node_modules", ".venv", "__pycache__"}

func stripBOM(data []byte) []byte {
	if len(data) >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
		return data[3:]
	}
	return data
}

func SanitizerDir() string {
	return filepath.Join(os.Getenv("USERPROFILE"), ".claude", "sanitizer")
}

func SecretsPath() string {
	return filepath.Join(SanitizerDir(), "sanitizer.json")
}

func LoadConfig() (*Config, error) {
	return LoadConfigFrom(SecretsPath())
}

func LoadConfigFrom(path string) (*Config, error) {
	cfg := &Config{
		MappingsManual:   make(map[string]string),
		MappingsAuto:     make(map[string]string),
		SkipPaths:     DefaultSkipPaths,
		HostnamePatterns: []string{},
		UnsanitizedPath:  "~/.claude/unsanitized/{project}",
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil
		}
		return nil, err
	}

	data = stripBOM(data)

	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, err
	}

	if cfg.MappingsManual == nil {
		cfg.MappingsManual = make(map[string]string)
	}
	if cfg.MappingsAuto == nil {
		cfg.MappingsAuto = make(map[string]string)
	}

	return cfg, nil
}

func (c *Config) AllMappings() map[string]string {
	all := make(map[string]string)
	for k, v := range c.MappingsAuto {
		all[k] = v
	}
	for k, v := range c.MappingsManual {
		all[k] = v
	}
	return all
}

func (c *Config) MergeAutoMappings(new map[string]string) map[string]string {
	merged := make(map[string]string)
	for k, v := range c.MappingsAuto {
		merged[k] = v
	}
	for k, v := range new {
		merged[k] = v
	}
	return merged
}

func (c *Config) ReverseMappings() map[string]string {
	reverse := make(map[string]string)
	for unsanitized, sanitized := range c.AllMappings() {
		reverse[sanitized] = unsanitized
	}
	return reverse
}

func (c *Config) ExpandUnsanitizedPath(projectName string) string {
	path := c.UnsanitizedPath
	if path == "" {
		path = "~/.claude/unsanitized/{project}"
	}
	if len(path) > 0 && path[0] == '~' {
		path = filepath.Join(os.Getenv("USERPROFILE"), path[1:])
	}
	return filepath.Clean(stringReplace(path, "{project}", projectName))
}

func stringReplace(s, old, new string) string {
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

func SaveAutoMappings(autoMappings map[string]string) error {
	return SaveAutoMappingsTo(SecretsPath(), autoMappings)
}

func SaveAutoMappingsTo(path string, autoMappings map[string]string) error {
	data, err := os.ReadFile(path)
	if err != nil && !os.IsNotExist(err) {
		return err
	}

	data = stripBOM(data)

	var raw map[string]any
	if len(data) > 0 {
		if err := json.Unmarshal(data, &raw); err != nil {
			return err
		}
	} else {
		raw = make(map[string]any)
	}

	raw["mappingsAuto"] = autoMappings

	out, err := json.MarshalIndent(raw, "", "    ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, out, 0644)
}

func InitializeConfigIfNeeded() error {
	path := SecretsPath()
	if _, err := os.Stat(path); err == nil {
		return nil
	}

	os.MkdirAll(SanitizerDir(), 0755)
	os.MkdirAll(filepath.Join(os.Getenv("USERPROFILE"), ".claude", "unsanitized"), 0755)

	cfg := map[string]any{
		"hostnamePatterns": []string{"\\.domain\\.local$"},
		"mappingsAuto":     map[string]string{},
		"mappingsManual": map[string]string{
			"server.domain.local":    "server.example.test",
			"111.91.241.85":          "111.50.100.1",
			"C:\\Users\\realuser":    "C:\\Users\\exampleuser",
			"secretproject":          "projectname",
		},
		"skipPaths":        DefaultSkipPaths,
		"unsanitizedPath":  "~/.claude/unsanitized/{project}",
	}

	data, err := json.MarshalIndent(cfg, "", "    ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}
