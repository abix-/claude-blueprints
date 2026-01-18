package internal

import (
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

func SanitizeTextWithFallback(text string, mappings map[string]string) string {
	text = SanitizeText(text, mappings)
	text = SanitizeIPs(text)
	return text
}

func UnsanitizeText(text string, reverseMappings map[string]string) string {
	return SanitizeText(text, reverseMappings)
}
