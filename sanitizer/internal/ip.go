// ip.go - IP address and hostname detection and sanitization.
// Uses deterministic hashing so the same real value always maps to the same fake value.
package internal

import (
	"crypto/md5"
	"fmt"
	"regexp"
)

// Package-level variables initialized once at startup.
// var (...) groups related declarations. MustCompile panics if regex is invalid
// (caught at startup, not runtime).
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

// NewSanitizedIP generates a deterministic fake IP for a real IP.
// Uses MD5 hash so same input always produces same output. This ensures:
// - Consistent mappings across sessions (no random drift)
// - Same IP in output matches same IP discovered in files
//
// All sanitized IPs use 111.x.x.x range (excluded from sanitization to prevent
// double-sanitizing). Octets are 1-254 to avoid .0 and .255.
func NewSanitizedIP(realIP string) string {
	// Prefix "ip:" prevents collision with hostname hashes
	hash := md5.Sum([]byte("ip:" + realIP))
	// Use first 3 bytes of hash for octets 2-4. Mod 254 + 1 gives range 1-254.
	b2 := int(hash[0])%254 + 1
	b3 := int(hash[1])%254 + 1
	b4 := int(hash[2])%254 + 1
	return fmt.Sprintf("111.%d.%d.%d", b2, b3, b4)
}

// SanitizeIPs finds all IPs in text and replaces non-excluded ones.
// Used as fallback for command output that might contain IPs not in mappings.
func SanitizeIPs(text string) string {
	// ReplaceAllStringFunc calls the function for each match.
	// Like PowerShell: [regex]::Replace($text, $pattern, { param($m) ... })
	return ipv4Regex.ReplaceAllStringFunc(text, func(ip string) string {
		if IsExcludedIP(ip) {
			return ip
		}
		return NewSanitizedIP(ip)
	})
}

// NewSanitizedHostname generates a deterministic fake hostname.
// Example: "server.domain.local" -> "host-a1b2c3d4.example.test"
func NewSanitizedHostname(realHostname string) string {
	hash := md5.Sum([]byte("host:" + realHostname))
	// %x formats as hex. hash[:4] takes first 4 bytes = 8 hex chars.
	return fmt.Sprintf("host-%x.example.test", hash[:4])
}

// IPv4Regex returns the compiled regex for external use (e.g., DiscoverSensitiveValues).
func IPv4Regex() *regexp.Regexp {
	return ipv4Regex
}
