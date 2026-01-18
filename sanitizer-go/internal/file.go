package internal

import (
	"io"
	"os"
	"path/filepath"
	"strings"
)

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

func IsExcludedPath(relativePath string, excludePaths []string) bool {
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

func SyncDir(srcDir, dstDir string, excludePaths []string, transform func(string) string) error {
	return filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}

		relPath, err := filepath.Rel(srcDir, path)
		if err != nil {
			return nil
		}

		if IsExcludedPath(relPath, excludePaths) {
			return nil
		}

		if info.Size() > 10*1024*1024 {
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
