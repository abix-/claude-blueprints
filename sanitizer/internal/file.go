package internal

import (
	"io"
	"os"
	"path/filepath"
	"strings"
)

const MaxFileSize = 10 * 1024 * 1024 // 10MB

func IsBinary(path string) bool {
	f, err := os.Open(path)
	if err != nil {
		return true
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

func ShouldProcessFile(path string, info os.FileInfo, projectPath string, skipPaths []string) bool {
	if info.IsDir() || info.Size() == 0 || info.Size() > MaxFileSize {
		return false
	}
	relPath, err := filepath.Rel(projectPath, path)
	if err != nil || strings.HasPrefix(relPath, "..") {
		return false
	}
	if IsSkippedPath(relPath, skipPaths) {
		return false
	}
	return !IsBinary(path)
}

func IsSkippedPath(relativePath string, skipPaths []string) bool {
	relativePath = strings.ReplaceAll(relativePath, "\\", "/")

	for _, skip := range skipPaths {
		skip = strings.ReplaceAll(skip, "\\", "/")
		if relativePath == skip ||
			strings.HasPrefix(relativePath, skip+"/") ||
			strings.Contains(relativePath, "/"+skip+"/") {
			return true
		}
	}
	return false
}

func SyncDir(srcDir, dstDir string, skipPaths []string, transform func(string) string) error {
	return filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}

		relPath, err := filepath.Rel(srcDir, path)
		if err != nil {
			return nil
		}

		if IsSkippedPath(relPath, skipPaths) {
			return nil
		}

		if info.Size() > MaxFileSize {
			return nil
		}

		dstPath := filepath.Join(dstDir, relPath)

		if err := os.MkdirAll(filepath.Dir(dstPath), 0755); err != nil {
			return nil
		}

		if IsBinary(path) {
			return copyFile(path, dstPath)
		}

		content, err := os.ReadFile(path)
		if err != nil {
			return copyFile(path, dstPath)
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
