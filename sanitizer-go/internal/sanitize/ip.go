package sanitize

import (
	"crypto/md5"
	"fmt"
	"math/rand"
	"regexp"
)

var (
	ipv4Regex = regexp.MustCompile(`\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b`)

	excludePatterns = []*regexp.Regexp{
		regexp.MustCompile(`^127\.`),
		regexp.MustCompile(`^0\.0\.0\.0$`),
		regexp.MustCompile(`^255\.255\.255\.255$`),
		regexp.MustCompile(`^169\.254\.`),
		regexp.MustCompile(`^224\.`),
		regexp.MustCompile(`^239\.`),
		regexp.MustCompile(`^11\.\d+\.\d+\.\d+$`),
	}
)

func isExcludedIP(ip string) bool {
	for _, pattern := range excludePatterns {
		if pattern.MatchString(ip) {
			return true
		}
	}
	return false
}

func deterministicFakeIP(realIP string) string {
	hash := md5.Sum([]byte("ip:" + realIP))
	b2 := int(hash[0])%254 + 1
	b3 := int(hash[1])%254 + 1
	b4 := int(hash[2])%254 + 1
	return fmt.Sprintf("11.%d.%d.%d", b2, b3, b4)
}

// SanitizeIPs replaces real IPs with deterministic fake IPs in the 11.x.x.x range.
func SanitizeIPs(text string) string {
	return ipv4Regex.ReplaceAllStringFunc(text, func(ip string) string {
		if isExcludedIP(ip) {
			return ip
		}
		return deterministicFakeIP(ip)
	})
}

// FindIPs returns all non-excluded IPs in text
func FindIPs(text string) []string {
	matches := ipv4Regex.FindAllString(text, -1)
	var result []string
	seen := make(map[string]bool)
	for _, ip := range matches {
		if !isExcludedIP(ip) && !seen[ip] {
			seen[ip] = true
			result = append(result, ip)
		}
	}
	return result
}

// NewFakeIP generates a random fake IP in 11.x.x.x range
func NewFakeIP() string {
	b2 := rand.Intn(254) + 1
	b3 := rand.Intn(254) + 1
	b4 := rand.Intn(254) + 1
	return fmt.Sprintf("11.%d.%d.%d", b2, b3, b4)
}

// NewFakeHostname generates a random fake hostname
func NewFakeHostname() string {
	const chars = "abcdefghijklmnopqrstuvwxyz0123456789"
	suffix := make([]byte, 8)
	for i := range suffix {
		suffix[i] = chars[rand.Intn(len(chars))]
	}
	return fmt.Sprintf("host-%s.example.test", string(suffix))
}

// IsExcludedIP checks if IP should be excluded (exported for use elsewhere)
func IsExcludedIP(ip string) bool {
	return isExcludedIP(ip)
}

// IPv4Regex returns the compiled regex for external use
func IPv4Regex() *regexp.Regexp {
	return ipv4Regex
}
