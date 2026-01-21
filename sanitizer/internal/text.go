// text.go - Text transformation: sanitize, unsanitize, and discover sensitive values.
package internal

import (
	"regexp"
	"sort"
	"strings"
)

// SanitizeText replaces all occurrences of mapping keys with their values.
// Processes longest keys first to handle overlapping strings correctly.
// Example: if mappings has both "111.14.33.209" and "111.135.16.104", we must
// replace "111.135.16.104" first, otherwise "111.14.33.209" would match and
// leave a stray "0" behind.
func SanitizeText(text string, mappings map[string]string) string {
	if len(mappings) == 0 {
		return text
	}

	// Extract keys into slice for sorting.
	// make([]string, 0, len(mappings)) pre-allocates capacity = slight perf gain.
	keys := make([]string, 0, len(mappings))
	for k := range mappings {
		keys = append(keys, k)
	}

	// Sort by length descending. sort.Slice takes a "less" function.
	// Returns true if keys[i] should come before keys[j].
	sort.Slice(keys, func(i, j int) bool {
		return len(keys[i]) > len(keys[j])
	})

	for _, key := range keys {
		text = strings.ReplaceAll(text, key, mappings[key])
	}
	return text
}

// UnsanitizeText reverses sanitization. Same algorithm, just pass reversed mappings.
// reverseMappings: sanitized -> original (opposite of normal mappings).
func UnsanitizeText(text string, reverseMappings map[string]string) string {
	return SanitizeText(text, reverseMappings)
}

// DiscoverSensitiveValues scans text for IPs and hostnames not yet in mappings.
// Returns new mappings only - caller should merge with existing and save.
//
// Generates random sanitized values with collision detection - no two real
// values will map to the same sanitized value.
func DiscoverSensitiveValues(text string, cfg *Config) map[string]string {
	discovered := make(map[string]string)

	// Track all used sanitized values to prevent collisions
	usedValues := make(map[string]bool)
	for _, v := range cfg.MappingsManual {
		usedValues[v] = true
	}
	for _, v := range cfg.MappingsAuto {
		usedValues[v] = true
	}

	// Find all IPv4 addresses
	ipRegex := IPv4Regex()
	for _, ip := range ipRegex.FindAllString(text, -1) {
		if !IsExcludedIP(ip) {
			if _, exists := cfg.MappingsManual[ip]; !exists {
				if _, exists := cfg.MappingsAuto[ip]; !exists {
					if _, exists := discovered[ip]; !exists {
						// Retry until we get a unique sanitized value
						sanitized := NewSanitizedIP()
						for usedValues[sanitized] {
							sanitized = NewSanitizedIP()
						}
						discovered[ip] = sanitized
						usedValues[sanitized] = true
					}
				}
			}
		}
	}

	// Find hostnames matching configured patterns.
	// Matches both standalone server names and FQDNs.
	// Pattern can match the server name portion; optional domain suffix is captured.
	for _, pattern := range cfg.HostnamePatterns {
		// Match pattern with optional domain suffix (e.g., .domain.local)
		re, err := regexp.Compile(`(?i)\b` + pattern + `(?:\.[a-zA-Z0-9-]+)*`)
		if err != nil {
			continue
		}
		for _, match := range re.FindAllString(text, -1) {
			if _, exists := cfg.MappingsManual[match]; !exists {
				if _, exists := cfg.MappingsAuto[match]; !exists {
					if _, exists := discovered[match]; !exists {
						sanitized := NewSanitizedHostname()
						for usedValues[sanitized] {
							sanitized = NewSanitizedHostname()
						}
						discovered[match] = sanitized
						usedValues[sanitized] = true
					}
				}
			}
		}
	}

	return discovered
}
