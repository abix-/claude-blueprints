// file.go - File system operations: binary detection, path filtering, directory sync.
package internal

import (
	"io"
	"os"
	"path/filepath"
	"strings"
)

const MaxFileSize = 10 * 1024 * 1024 // 10MB - skip large files to avoid memory issues

// IsBinary checks if a file is binary by looking for null bytes in first 8KB.
// Returns true (assume binary) on any read error - safer to skip than corrupt.
func IsBinary(path string) bool {
	f, err := os.Open(path)
	if err != nil {
		return true
	}
	defer f.Close() // Runs when function exits, like try/finally

	buf := make([]byte, 8192)
	n, err := f.Read(buf)
	// io.EOF is normal end-of-file, not an error. Other errors = assume binary.
	if err != nil && err != io.EOF {
		return true
	}

	// Null byte = binary file (images, executables, etc.)
	for i := 0; i < n; i++ {
		if buf[i] == 0 {
			return true
		}
	}
	return false
}

// ShouldProcessFile determines if a file should be sanitized.
// Skips: directories, empty files, large files, symlinks, binary files, excluded paths.
func ShouldProcessFile(path string, info os.FileInfo, projectPath string, skipPaths []string) bool {
	if info.IsDir() || info.Size() == 0 || info.Size() > MaxFileSize {
		return false
	}

	// Bitwise AND to check mode flags. os.ModeSymlink is a bit flag.
	// Like PowerShell: ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
	if info.Mode()&os.ModeSymlink != 0 {
		return false
	}

	// Ensure file is under project directory (prevent path traversal)
	relPath, err := filepath.Rel(projectPath, path)
	if err != nil || strings.HasPrefix(relPath, "..") {
		return false
	}

	// Always skip .claude directory (config and unsanitized data)
	normalizedRel := strings.ReplaceAll(relPath, "\\", "/")
	if strings.HasPrefix(normalizedRel, ".claude/") || normalizedRel == ".claude" {
		return false
	}

	if IsSkippedPath(relPath, skipPaths) {
		return false
	}

	return !IsBinary(path)
}

// IsSkippedPath checks if path matches any skip pattern (.git, node_modules, etc.)
// Matches: exact name, starts with pattern/, contains /pattern/
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

// SyncDir copies srcDir to dstDir, optionally transforming text file content.
// Binary files are copied as-is. Used to sync working tree <-> unsanitized directory.
//
// transform is a function that takes file content and returns modified content.
// Pass nil to copy without modification. Example: pass UnsanitizeText to restore
// original values when syncing to unsanitized directory.
func SyncDir(srcDir, dstDir string, skipPaths []string, transform func(string) string) error {
	// filepath.Walk recursively visits all files/dirs. Like Get-ChildItem -Recurse.
	// The callback function is called for each item. Return nil to continue,
	// return error to stop walking.
	return filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil // Skip errors and directories, continue walking
		}

		if info.Mode()&os.ModeSymlink != 0 {
			return nil // Skip symlinks
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
			return nil // Continue on mkdir failure
		}

		// Binary files: copy bytes directly, no transformation
		if IsBinary(path) {
			return copyFile(path, dstPath)
		}

		// Text files: read, transform, write
		content, err := os.ReadFile(path)
		if err != nil {
			return copyFile(path, dstPath) // Fallback to binary copy on read error
		}

		transformed := content
		if transform != nil {
			transformed = []byte(transform(string(content)))
		}

		return os.WriteFile(dstPath, transformed, info.Mode())
	})
}

// copyFile does a binary copy using io.Copy (streams, doesn't load entire file).
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
