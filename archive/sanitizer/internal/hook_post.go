// hook_post.go - PostToolUse hook for sanitizing tool output.
// Catches sensitive values in Grep/Glob output that might have been missed
// (new values added after session start, files modified outside Claude, etc.)
package internal

import (
	"encoding/json"
)

// HookPostToolUse processes tool output after execution.
// Scans output for sensitive values and replaces them before Claude sees the result.
//
// Configure in settings.json:
//
//	"PostToolUse": [{
//	  "matcher": "Grep|Glob",
//	  "hooks": [{ "type": "command", "command": "sanitizer.exe hook-post" }]
//	}]
func HookPostToolUse(input []byte) ([]byte, error) {
	var hookData struct {
		HookEventName string `json:"hook_event_name"`
		ToolName      string `json:"tool_name"`
		ToolOutput    string `json:"tool_output"`
	}

	if err := json.Unmarshal(input, &hookData); err != nil {
		return nil, nil
	}

	if hookData.HookEventName != "PostToolUse" {
		return nil, nil
	}

	if hookData.ToolOutput == "" {
		return nil, nil
	}

	cfg, err := LoadConfig()
	if err != nil {
		return nil, nil
	}

	// Discover any new sensitive values in the output
	discovered := DiscoverSensitiveValues(hookData.ToolOutput, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	// Save new mappings if we found any
	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	// Sanitize the output
	allMappings := cfg.BuildAllMappings(autoMappings)
	sanitized := SanitizeText(hookData.ToolOutput, allMappings)

	// No changes needed
	if sanitized == hookData.ToolOutput {
		return nil, nil
	}

	// Return modified output
	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName": "PostToolUse",
			"updatedOutput": sanitized,
		},
	})
}
