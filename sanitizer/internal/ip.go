// ip.go - IP address and hostname detection and sanitization.
// Uses random generation - caller saves to mappingsAuto for consistency.
package internal

import (
	"fmt"
	"math/rand"
	"regexp"
)

// Package-level variables initialized once at startup.
var (
	// Matches valid IPv4 addresses (0-255 in each octet).
	// \b = word boundary to avoid matching "1.2.3.4" inside "11.2.3.45"
	ipv4Regex = regexp.MustCompile(`\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b`)

	// IPs that should NOT be sanitized - infrastructure/reserved addresses
	excludeIPPatterns = []*regexp.Regexp{
		regexp.MustCompile(`^127\.`),              // loopback (localhost)
		regexp.MustCompile(`^0\.0\.0\.0$`),        // unspecified/any
		regexp.MustCompile(`^255\.`),              // subnet masks
		regexp.MustCompile(`^169\.254\.`),         // link-local (APIPA)
		regexp.MustCompile(`^2(2[4-9]|3[0-9])\.`), // multicast 224.x-239.x
		regexp.MustCompile(`^111\.`),              // our sanitized IP range
	}
)

// IsExcludedIP returns true if this IP should NOT be sanitized.
func IsExcludedIP(ip string) bool {
	for _, pattern := range excludeIPPatterns {
		if pattern.MatchString(ip) {
			return true
		}
	}
	return false
}

// NewSanitizedIP generates a random fake IP in the 111.x.x.x range.
// Caller must save the mapping to mappingsAuto for consistency across sessions.
// Octets are 1-254 to avoid .0 (network) and .255 (broadcast).
func NewSanitizedIP() string {
	b2 := rand.Intn(254) + 1
	b3 := rand.Intn(254) + 1
	b4 := rand.Intn(254) + 1
	return fmt.Sprintf("111.%d.%d.%d", b2, b3, b4)
}

// NewSanitizedHostname generates a random fake hostname.
// Caller must save the mapping to mappingsAuto for consistency.
func NewSanitizedHostname() string {
	const chars = "abcdefghijklmnopqrstuvwxyz0123456789"
	suffix := make([]byte, 8)
	for i := range suffix {
		suffix[i] = chars[rand.Intn(len(chars))]
	}
	return fmt.Sprintf("host-%s.example.test", string(suffix))
}

// IPv4Regex returns the compiled regex for external use.
func IPv4Regex() *regexp.Regexp {
	return ipv4Regex
}
