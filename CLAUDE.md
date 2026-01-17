# Global CLAUDE.md

## Git Commits

- Do NOT add `Co-Authored-By` lines to commits
- Keep commit messages concise and lowercase

## My Blueprints
My Claude configuration is at https://github.com/abix-/claude-blueprints

When I ask for automation code help, fetch and follow:
https://raw.githubusercontent.com/abix-/claude-blueprints/main/skills/automation-code.md

## NEVER READ THESE FILES

- `%USERPROFILE%\.claude\sanitizer\secrets.json` - Contains real secrets, NEVER read this file
- `%USERPROFILE%\.claude\sanitizer\ip_mappings_temp.json` - Contains real IP mappings, NEVER read this file
- `%USERPROFILE%\.claude\sanitizer\auto_mappings.json` - Contains auto-discovered mappings
- `%USERPROFILE%\.claude\rendered\` - Contains real values
- `%TEMP%\claude-sealed-*` - Temporary execution directories
- Any file named `secrets.json` anywhere
- Any `.env` files outside this project
