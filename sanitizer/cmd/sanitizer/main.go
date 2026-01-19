// main.go - CLI entry point for the sanitizer.
// Single binary with subcommands, invoked by Claude Code hooks.
//
// Subcommands:
//   - hook-session-start: Bulk sanitize project at session start
//   - hook-session-stop:  Sync to unsanitized directory at session end
//   - hook-file-access:   PreToolUse hook for Read/Edit/Write tools
//   - hook-bash:          PreToolUse hook for Bash tool
//   - hook-post:          PostToolUse hook for sanitizing tool output
//   - exec:               Run command with unsanitized values
//   - sanitize-ips:       Pipe filter for IP sanitization
package main

import (
	"fmt"
	"io"
	"os"

	"github.com/abix-/claude-blueprints/sanitizer/internal"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: sanitizer <command>")
		fmt.Fprintln(os.Stderr, "commands: sanitize-ips, hook-file-access, hook-bash, hook-post, hook-session-start, hook-session-stop, exec")
		os.Exit(1)
	}

	// switch on first argument = subcommand pattern.
	// Each case routes to the appropriate handler function.
	switch os.Args[1] {
	case "sanitize-ips":
		runSanitizeIPs()
	case "hook-file-access":
		runHook(internal.HookFileAccess)
	case "hook-bash":
		runHook(internal.HookBash)
	case "hook-post":
		runHook(internal.HookPostToolUse)
	case "hook-session-start":
		runSessionHook(internal.SessionStartCmd)
	case "hook-session-stop":
		runSessionHook(internal.SessionStopCmd)
	case "exec":
		runExec()
	default:
		fmt.Fprintf(os.Stderr, "unknown command: %s\n", os.Args[1])
		os.Exit(1)
	}
}

func readStdin() ([]byte, error) {
	return io.ReadAll(os.Stdin)
}

// runSanitizeIPs is a pipe filter: stdin -> discover & sanitize -> stdout.
// Uses config for mappings, saves new discoveries. Useful for testing.
func runSanitizeIPs() {
	input, err := readStdin()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading stdin: %v\n", err)
		os.Exit(1)
	}

	cfg, err := internal.LoadConfig()
	if err != nil {
		fmt.Fprintf(os.Stderr, "config error: %v\n", err)
		os.Exit(1)
	}

	text := string(input)
	discovered := internal.DiscoverSensitiveValues(text, cfg)
	if len(discovered) > 0 {
		autoMappings := cfg.MergeAutoMappings(discovered)
		internal.SaveAutoMappings(autoMappings)
		cfg.MappingsAuto = autoMappings
	}

	fmt.Print(internal.SanitizeText(text, cfg.AllMappings()))
}

// runHook handles PreToolUse/PostToolUse hooks.
// Claude Code sends JSON on stdin, expects JSON (or nothing) on stdout.
// Exit 0 even on error = fail open (allow the operation).
func runHook(fn func([]byte) ([]byte, error)) {
	input, err := readStdin()
	if err != nil {
		fmt.Fprintf(os.Stderr, "stdin error: %v\n", err)
		os.Exit(0) // Fail open
	}

	output, err := fn(input)
	if err != nil {
		fmt.Fprintf(os.Stderr, "hook error: %v\n", err)
		os.Exit(0) // Fail open
	}

	// nil output = no modification (allow as-is)
	// non-nil = JSON response to modify/block the operation
	if output != nil {
		fmt.Print(string(output))
	}
}

// runSessionHook handles session start/stop hooks.
// These don't read stdin or produce output - they just do work.
func runSessionHook(fn func() error) {
	if err := fn(); err != nil {
		fmt.Fprintf(os.Stderr, "session hook error: %v\n", err)
	}
}

// runExec handles the "exec" subcommand for running commands with real values.
// Usage: sanitizer exec '<command>'
func runExec() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: sanitizer exec <command>")
		os.Exit(1)
	}
	if err := internal.Exec(os.Args[2]); err != nil {
		fmt.Fprintf(os.Stderr, "exec error: %v\n", err)
		os.Exit(1)
	}
}
