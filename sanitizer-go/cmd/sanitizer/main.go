package main

import (
	"bufio"
	"fmt"
	"io"
	"os"

	"github.com/abix-/claude-blueprints/sanitizer-go/internal/scrub"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: sanitizer <command>")
		fmt.Fprintln(os.Stderr, "commands: scrub-ips")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "scrub-ips":
		scrubIPs()
	default:
		fmt.Fprintf(os.Stderr, "unknown command: %s\n", os.Args[1])
		os.Exit(1)
	}
}

func scrubIPs() {
	reader := bufio.NewReader(os.Stdin)
	text, err := io.ReadAll(reader)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading stdin: %v\n", err)
		os.Exit(1)
	}
	fmt.Print(scrub.ScrubIPs(string(text)))
}
