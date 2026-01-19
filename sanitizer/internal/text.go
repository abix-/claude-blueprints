// text.go - Text transformation: sanitize, unsanitize, and discover sensitive values.
package internal

import (
	"regexp"
	"sort"
	"strings"
)

// SanitizeText replaces all occurrences of mapping keys with their values.
// Processes longest keys first to handle overlapping strings correctly.
// Example: if mappings has both "192.168.1.1" and "192.168.1.10", we must
// replace "192.168.1.10" first, otherwise "192.168.1.1" would match and
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

// sanitizeTextWithFallback applies mappings, then catches any unmapped IPs.
// Used for command output where we might see IPs that weren't in the original files.
func sanitizeTextWithFallback(text string, mappings map[string]string) string {
	text = SanitizeText(text, mappings)
	text = SanitizeIPs(text) // Catch any IPs not in mappings
	return text
}

// UnsanitizeText reverses sanitization. Same algorithm, just pass reversed mappings.
// reverseMappings: sanitized -> original (opposite of normal mappings).
func UnsanitizeText(text string, reverseMappings map[string]string) string {
	return SanitizeText(text, reverseMappings)
}

// DiscoverSensitiveValues scans text for IPs and hostnames not yet in mappings.
// Returns new mappings only - caller should merge with existing.
//
// Skips values that already exist in manual or auto mappings to avoid
// regenerating (and potentially changing) existing sanitized values.
func DiscoverSensitiveValues(text string, cfg *Config) map[string]string {
	discovered := make(map[string]string)

	// Find all IPv4 addresses
	ipRegex := IPv4Regex()
	for _, ip := range ipRegex.FindAllString(text, -1) {
		if !IsExcludedIP(ip) {
			// "comma ok" idiom: _, exists := map[key] returns (value, bool).
			// We only care about existence, so discard value with _.
			// Like PowerShell: if (-not $hash.ContainsKey($key))
			if _, exists := cfg.MappingsManual[ip]; !exists {
				if _, exists := cfg.MappingsAuto[ip]; !exists {
					if _, exists := discovered[ip]; !exists {
						discovered[ip] = NewSanitizedIP(ip)
					}
				}
			}
		}
	}

	// Find hostnames matching configured patterns (e.g., \.domain\.local$)
	for _, pattern := range cfg.HostnamePatterns {
		// Build regex: hostname chars + user's pattern.
		// (?i) = case insensitive. Matches "server.domain.local", "DB.DOMAIN.LOCAL", etc.
		re, err := regexp.Compile(`(?i)[a-zA-Z0-9][-a-zA-Z0-9\.]*` + pattern)
		if err != nil {
			continue // Invalid pattern - skip silently
		}
		for _, match := range re.FindAllString(text, -1) {
			if _, exists := cfg.MappingsManual[match]; !exists {
				if _, exists := cfg.MappingsAuto[match]; !exists {
					if _, exists := discovered[match]; !exists {
						discovered[match] = NewSanitizedHostname(match)
					}
				}
			}
		}
	}

	return discovered
}
