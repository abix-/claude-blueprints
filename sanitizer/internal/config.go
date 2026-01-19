// config.go - Configuration loading, saving, and mapping management.
// Handles sanitizer.json which stores manual/auto mappings between real and sanitized values.
package internal

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Config holds all sanitizer settings.
// The `json:"..."` tags tell Go how to map struct fields to JSON keys.
// In PowerShell terms: like [Parameter()] attributes but for serialization.
type Config struct {
	MappingsManual   map[string]string `json:"mappingsManual"`
	MappingsAuto     map[string]string `json:"mappingsAuto"`
	SkipPaths        []string          `json:"skipPaths"`
	HostnamePatterns []string          `json:"hostnamePatterns"`
	UnsanitizedPath  string            `json:"unsanitizedPath"`
}

var DefaultSkipPaths = []string{".git", "node_modules", ".venv", "__pycache__"}

// stripBOM removes UTF-8 BOM that Windows apps (notepad, VS Code) add to files.
// Without this, json.Unmarshal fails on files saved with BOM.
func stripBOM(data []byte) []byte {
	if len(data) >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
		return data[3:]
	}
	return data
}

func SanitizerDir() string {
	return filepath.Join(os.Getenv("USERPROFILE"), ".claude", "sanitizer")
}

func ConfigPath() string {
	return filepath.Join(SanitizerDir(), "sanitizer.json")
}

func LoadConfig() (*Config, error) {
	return LoadConfigFrom(ConfigPath())
}

// LoadConfigFrom reads config from disk, returning defaults if file doesn't exist.
// Go pattern: return (value, error) - caller must check error before using value.
// Unlike PowerShell's try/catch, errors are explicit return values.
func LoadConfigFrom(path string) (*Config, error) {
	// &Config{} creates a pointer to a new Config struct.
	// make() initializes maps - without it, maps are nil and assignments panic.
	cfg := &Config{
		MappingsManual:   make(map[string]string),
		MappingsAuto:     make(map[string]string),
		SkipPaths:        DefaultSkipPaths,
		HostnamePatterns: []string{},
		UnsanitizedPath:  "~/.claude/unsanitized/{project}",
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil // File doesn't exist = use defaults, not an error
		}
		return nil, err
	}

	data = stripBOM(data)

	// json.Unmarshal populates cfg in-place. If JSON has a field, it overwrites
	// the default. If JSON is missing a field, the default remains.
	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, err
	}

	// JSON null or missing field results in nil map, which panics on assignment.
	// Re-initialize if needed.
	if cfg.MappingsManual == nil {
		cfg.MappingsManual = make(map[string]string)
	}
	if cfg.MappingsAuto == nil {
		cfg.MappingsAuto = make(map[string]string)
	}

	return cfg, nil
}

// AllMappings merges auto + manual mappings. Manual wins on conflict.
// (c *Config) is a "receiver" - makes this a method on Config type.
// Like PowerShell: $config.AllMappings() instead of Get-AllMappings -Config $config
func (c *Config) AllMappings() map[string]string {
	all := make(map[string]string)
	// Auto mappings first (lower priority)
	for k, v := range c.MappingsAuto {
		all[k] = v
	}
	// Manual mappings overwrite (higher priority)
	for k, v := range c.MappingsManual {
		all[k] = v
	}
	return all
}

// MergeAutoMappings combines existing auto mappings with newly discovered ones.
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

// ReverseMappings flips keys/values for unsanitizing (sanitized -> original).
func (c *Config) ReverseMappings() map[string]string {
	reverse := make(map[string]string)
	for unsanitized, sanitized := range c.AllMappings() {
		reverse[sanitized] = unsanitized
	}
	return reverse
}

// ExpandUnsanitizedPath resolves ~ and {project} placeholders.
func (c *Config) ExpandUnsanitizedPath(projectName string) string {
	path := c.UnsanitizedPath
	if path == "" {
		path = "~/.claude/unsanitized/{project}"
	}
	if len(path) > 0 && path[0] == '~' {
		path = filepath.Join(os.Getenv("USERPROFILE"), path[1:])
	}
	return filepath.Clean(strings.Replace(path, "{project}", projectName, 1))
}

// BuildAllMappings creates final mapping set from provided auto + config's manual.
// Used when we have fresh auto-mappings that haven't been saved to config yet.
func (c *Config) BuildAllMappings(autoMappings map[string]string) map[string]string {
	all := make(map[string]string)
	for k, v := range autoMappings {
		all[k] = v
	}
	for k, v := range c.MappingsManual {
		all[k] = v
	}
	return all
}

func SaveAutoMappings(autoMappings map[string]string) error {
	return SaveAutoMappingsTo(ConfigPath(), autoMappings)
}

func lockPath(configPath string) string {
	return configPath + ".lock"
}

// acquireLock creates an exclusive lock file. Uses O_EXCL flag which fails if
// file exists - atomic "create if not exists". Retries for 5 seconds, then
// assumes stale lock and forces.
func acquireLock(configPath string) (*os.File, error) {
	lock := lockPath(configPath)
	for i := 0; i < 50; i++ {
		// O_EXCL = fail if file exists (atomic check-and-create)
		f, err := os.OpenFile(lock, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0644)
		if err == nil {
			return f, nil
		}
		time.Sleep(100 * time.Millisecond)
	}
	// Stale lock (crashed process) - force remove and retry
	os.Remove(lock)
	return os.OpenFile(lock, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0644)
}

func releaseLock(f *os.File, configPath string) {
	f.Close()
	os.Remove(lockPath(configPath))
}

// SaveAutoMappingsTo persists auto-mappings while preserving other config fields.
// Uses file locking to prevent race conditions from concurrent sanitizer instances.
func SaveAutoMappingsTo(path string, autoMappings map[string]string) error {
	lock, err := acquireLock(path)
	if err != nil {
		return err
	}
	// defer = "run this when function exits" (like try/finally).
	// Guarantees lock release even if we return early on error.
	defer releaseLock(lock, path)

	data, err := os.ReadFile(path)
	if err != nil && !os.IsNotExist(err) {
		return err
	}

	data = stripBOM(data)

	// Unmarshal into map[string]any to preserve unknown fields.
	// If we used Config struct, we'd lose any extra fields user added.
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

// InitializeConfigIfNeeded creates default config if none exists.
func InitializeConfigIfNeeded() error {
	path := ConfigPath()
	// os.Stat returns error if file doesn't exist. No error = file exists = done.
	if _, err := os.Stat(path); err == nil {
		return nil
	}

	os.MkdirAll(SanitizerDir(), 0755)
	os.MkdirAll(filepath.Join(os.Getenv("USERPROFILE"), ".claude", "unsanitized"), 0755)

	// Example config with placeholder values user should replace
	cfg := map[string]any{
		"hostnamePatterns": []string{"\\.domain\\.local$"},
		"mappingsAuto":     map[string]string{},
		"mappingsManual": map[string]string{
			"server.domain.local": "server.example.test",
			"111.91.241.85":       "111.50.100.1",
			"C:\\Users\\realuser": "C:\\Users\\exampleuser",
			"secretproject":       "projectname",
		},
		"skipPaths":       DefaultSkipPaths,
		"unsanitizedPath": "~/.claude/unsanitized/{project}",
	}

	data, err := json.MarshalIndent(cfg, "", "    ")
	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}
