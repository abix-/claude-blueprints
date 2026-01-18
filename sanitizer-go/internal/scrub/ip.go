package scrub

import (
	"crypto/md5"
	"fmt"
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

// ScrubIPs replaces real IPs with deterministic fake IPs in the 11.x.x.x range.
func ScrubIPs(text string) string {
	return ipv4Regex.ReplaceAllStringFunc(text, func(ip string) string {
		if isExcludedIP(ip) {
			return ip
		}
		return deterministicFakeIP(ip)
	})
}
