package sanitize

import (
	"sort"
	"strings"
)

// Text applies mappings (real->fake) to text, longest keys first
func Text(text string, mappings map[string]string) string {
	if len(mappings) == 0 {
		return text
	}

	// Sort keys by length descending (longest first)
	keys := make([]string, 0, len(mappings))
	for k := range mappings {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		return len(keys[i]) > len(keys[j])
	})

	for _, real := range keys {
		text = strings.ReplaceAll(text, real, mappings[real])
	}
	return text
}

// TextWithFallback applies mappings then falls back to IP sanitization for unknowns
func TextWithFallback(text string, mappings map[string]string) string {
	text = Text(text, mappings)
	text = SanitizeIPs(text)
	return text
}

// Unsanitize applies reverse mappings (fake->real) to text
func Unsanitize(text string, reverseMappings map[string]string) string {
	return Text(text, reverseMappings)
}
