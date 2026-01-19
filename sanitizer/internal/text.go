package internal

import (
	"regexp"
	"sort"
	"strings"
)

func SanitizeText(text string, mappings map[string]string) string {
	if len(mappings) == 0 {
		return text
	}

	keys := make([]string, 0, len(mappings))
	for k := range mappings {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		return len(keys[i]) > len(keys[j])
	})

	for _, key := range keys {
		text = strings.ReplaceAll(text, key, mappings[key])
	}
	return text
}

func sanitizeTextWithFallback(text string, mappings map[string]string) string {
	text = SanitizeText(text, mappings)
	text = SanitizeIPs(text)
	return text
}

func UnsanitizeText(text string, reverseMappings map[string]string) string {
	return SanitizeText(text, reverseMappings)
}

func DiscoverSensitiveValues(text string, cfg *Config) map[string]string {
	discovered := make(map[string]string)

	ipRegex := IPv4Regex()
	for _, ip := range ipRegex.FindAllString(text, -1) {
		if !IsExcludedIP(ip) {
			if _, exists := cfg.MappingsManual[ip]; !exists {
				if _, exists := cfg.MappingsAuto[ip]; !exists {
					if _, exists := discovered[ip]; !exists {
						discovered[ip] = NewSanitizedIP(ip)
					}
				}
			}
		}
	}

	for _, pattern := range cfg.HostnamePatterns {
		re, err := regexp.Compile(`(?i)[a-zA-Z0-9][-a-zA-Z0-9\.]*` + pattern)
		if err != nil {
			continue
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
