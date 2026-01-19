package internal

import (
	"encoding/json"
)

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

	// Discover new sensitive values in output
	discovered := DiscoverSensitiveValues(hookData.ToolOutput, cfg)
	autoMappings := cfg.MergeAutoMappings(discovered)

	if len(autoMappings) > len(cfg.MappingsAuto) {
		SaveAutoMappings(autoMappings)
	}

	allMappings := cfg.BuildAllMappings(autoMappings)
	sanitized := SanitizeText(hookData.ToolOutput, allMappings)

	if sanitized == hookData.ToolOutput {
		return nil, nil
	}

	return json.Marshal(map[string]any{
		"hookSpecificOutput": map[string]any{
			"hookEventName":   "PostToolUse",
			"updatedOutput":   sanitized,
		},
	})
}
