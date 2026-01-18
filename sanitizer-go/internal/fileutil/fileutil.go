package fileutil

import (
	"io"
	"os"
	"path/filepath"
	"strings"
)

// IsBinary checks if file contains null bytes (binary indicator)
func IsBinary(path string) bool {
	f, err := os.Open(path)
	if err != nil {
		return true // assume binary on error
	}
	defer f.Close()

	buf := make([]byte, 8192)
	n, err := f.Read(buf)
	if err != nil && err != io.EOF {
		return true
	}

	for i := 0; i < n; i++ {
		if buf[i] == 0 {
			return true
		}
	}
	return false
}

// IsExcludedPath checks if path matches exclusion patterns
func IsExcludedPath(relativePath string, excludePaths []string) bool {
	// Normalize separators
	relativePath = strings.ReplaceAll(relativePath, "\\", "/")

	for _, exclude := range excludePaths {
		exclude = strings.ReplaceAll(exclude, "\\", "/")
		if relativePath == exclude ||
			strings.HasPrefix(relativePath, exclude+"/") ||
			strings.Contains(relativePath, "/"+exclude+"/") {
			return true
		}
	}
	return false
}

// SyncDir copies files from src to dst, applying transform to text files
func SyncDir(srcDir, dstDir string, excludePaths []string, transform func(string) string) error {
	return filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // skip errors
		}
		if info.IsDir() {
			return nil
		}

		relPath, err := filepath.Rel(srcDir, path)
		if err != nil {
			return nil
		}

		if IsExcludedPath(relPath, excludePaths) {
			return nil
		}

		// Skip large files (>10MB)
		if info.Size() > 10*1024*1024 {
			return nil
		}

		dstPath := filepath.Join(dstDir, relPath)

		// Ensure destination directory exists
		if err := os.MkdirAll(filepath.Dir(dstPath), 0755); err != nil {
			return nil
		}

		// Binary files: copy directly
		if IsBinary(path) {
			return copyFile(path, dstPath)
		}

		// Text files: transform content
		content, err := os.ReadFile(path)
		if err != nil {
			return copyFile(path, dstPath) // fallback to copy
		}

		transformed := content
		if transform != nil {
			transformed = []byte(transform(string(content)))
		}

		return os.WriteFile(dstPath, transformed, info.Mode())
	})
}

func copyFile(src, dst string) error {
	srcF, err := os.Open(src)
	if err != nil {
		return err
	}
	defer srcF.Close()

	dstF, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer dstF.Close()

	_, err = io.Copy(dstF, srcF)
	return err
}
