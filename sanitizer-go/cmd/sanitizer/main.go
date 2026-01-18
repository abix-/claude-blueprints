package main

import (
	"fmt"
	"io"
	"os"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal/hook"
	"github.com/abix-/claude-blueprints/sanitizer-go/internal/sanitize"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: sanitizer <command>")
		fmt.Fprintln(os.Stderr, "commands: sanitize-ips, hook-file-access")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "sanitize-ips":
		runSanitizeIPs()
	case "hook-file-access":
		runHook(hook.FileAccess)
	default:
		fmt.Fprintf(os.Stderr, "unknown command: %s\n", os.Args[1])
		os.Exit(1)
	}
}

func readStdin() ([]byte, error) {
	return io.ReadAll(os.Stdin)
}

func runSanitizeIPs() {
	input, err := readStdin()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading stdin: %v\n", err)
		os.Exit(1)
	}
	fmt.Print(sanitize.SanitizeIPs(string(input)))
}

func runHook(fn func([]byte) ([]byte, error)) {
	input, err := readStdin()
	if err != nil {
		os.Exit(0) // fail open
	}
	output, err := fn(input)
	if err != nil {
		os.Exit(0) // fail open
	}
	if output != nil {
		fmt.Print(string(output))
	}
}
