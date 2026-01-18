package main

import (
	"fmt"
	"io"
	"os"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: sanitizer <command>")
		fmt.Fprintln(os.Stderr, "commands: sanitize-ips, hook-file-access, hook-bash, hook-session-start, hook-session-stop")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "sanitize-ips":
		runSanitizeIPs()
	case "hook-file-access":
		runHook(internal.HookFileAccess)
	case "hook-bash":
		runHook(internal.HookBash)
	case "hook-session-start":
		runSessionHook(internal.SessionStartCmd)
	case "hook-session-stop":
		runSessionHook(internal.SessionStopCmd)
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
	fmt.Print(internal.SanitizeIPs(string(input)))
}

func runHook(fn func([]byte) ([]byte, error)) {
	input, err := readStdin()
	if err != nil {
		fmt.Fprintf(os.Stderr, "stdin error: %v\n", err)
		os.Exit(0)
	}
	output, err := fn(input)
	if err != nil {
		fmt.Fprintf(os.Stderr, "hook error: %v\n", err)
		os.Exit(0)
	}
	if output != nil {
		fmt.Print(string(output))
	}
}

func runSessionHook(fn func() error) {
	if err := fn(); err != nil {
		fmt.Fprintf(os.Stderr, "session hook error: %v\n", err)
	}
}
