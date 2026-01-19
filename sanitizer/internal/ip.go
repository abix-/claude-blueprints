package internal

import (
	"crypto/md5"
	"fmt"
	"regexp"
)

var (
	ipv4Regex = regexp.MustCompile(`\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b`)

	excludeIPPatterns = []*regexp.Regexp{
		regexp.MustCompile(`^127\.`),              // loopback
		regexp.MustCompile(`^0\.0\.0\.0$`),        // unspecified
		regexp.MustCompile(`^255\.`),              // subnet masks
		regexp.MustCompile(`^169\.254\.`),         // link-local
		regexp.MustCompile(`^2(2[4-9]|3[0-9])\.`), // multicast 224-239
		regexp.MustCompile(`^111\.`),              // sanitized IP range
	}
)

func IsExcludedIP(ip string) bool {
	for _, pattern := range excludeIPPatterns {
		if pattern.MatchString(ip) {
			return true
		}
	}
	return false
}

func NewSanitizedIP(realIP string) string {
	hash := md5.Sum([]byte("ip:" + realIP))
	b2 := int(hash[0])%254 + 1
	b3 := int(hash[1])%254 + 1
	b4 := int(hash[2])%254 + 1
	return fmt.Sprintf("111.%d.%d.%d", b2, b3, b4)
}

func SanitizeIPs(text string) string {
	return ipv4Regex.ReplaceAllStringFunc(text, func(ip string) string {
		if IsExcludedIP(ip) {
			return ip
		}
		return NewSanitizedIP(ip)
	})
}

func NewSanitizedHostname(realHostname string) string {
	hash := md5.Sum([]byte("host:" + realHostname))
	return fmt.Sprintf("host-%x.example.test", hash[:4])
}

func IPv4Regex() *regexp.Regexp {
	return ipv4Regex
}
