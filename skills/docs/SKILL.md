---
name: docs
description: Build, preview, and deploy MkDocs Material documentation sites. Use when working on docs, mkdocs.yml, or GitHub Pages.
version: "1.0"
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
---
# Docs

Build and maintain MkDocs Material documentation sites.

## Stack

- MkDocs Material (`pip install mkdocs-material`)
- GitHub Pages via `gh-pages` branch
- GitHub Actions auto-deploy on push to docs/ or mkdocs.yml

## Commands

| Command | What |
|---------|------|
| `mkdocs serve` | Local preview at localhost:8000 |
| `mkdocs build` | Build to site/ (check for warnings) |
| `mkdocs gh-deploy --force` | Manual deploy to GitHub Pages |

## mkdocs.yml patterns

```yaml
theme:
  name: material
  palette:
    scheme: slate          # dark mode
    primary: custom        # use extra.css for colors
    accent: custom
  features:
    - content.code.copy    # copy button on code blocks
    - content.tabs.link    # linked tabs across page
    - navigation.instant   # SPA-style navigation
    - navigation.sections  # collapsible nav sections
    - navigation.top       # back to top button
    - search.highlight     # highlight search terms
    - toc.follow           # TOC follows scroll

markdown_extensions:
  - admonition             # !!! note/warning/danger blocks
  - pymdownx.details       # collapsible ??? blocks
  - pymdownx.superfences   # fenced code in admonitions
  - pymdownx.tabbed:       # === "Tab" content tabs
      alternate_style: true
  - pymdownx.highlight     # syntax highlighting
  - tables                 # pipe tables
  - toc:
      permalink: true      # anchor links on headings

extra_css:
  - stylesheets/extra.css  # custom theme overrides
```

## Markdown features to use

### Admonitions (callout boxes)

```markdown
!!! tip "Title"
    Content indented 4 spaces.

!!! warning "Caution"
    Warning content.

!!! danger "Critical"
    Danger content.

!!! bug "Known issue"
    Bug description.

!!! info "Note"
    Informational content.
```

### Collapsible blocks (great for long JSON examples)

```markdown
??? example "Example response"        # collapsed by default

    ```json
    {"key": "value"}
    ```

???+ example "Example response"       # expanded by default

    ```json
    {"key": "value"}
    ```
```

### Content tabs (show alternatives side by side)

```markdown
=== "Python CLI"

    ```bash
    python tool.py command
    ```

=== "curl"

    ```bash
    curl http://localhost:8085/api/endpoint
    ```
```

## Custom theming (extra.css)

Override Material variables in `[data-md-color-scheme="slate"]`:

```css
[data-md-color-scheme="slate"] {
  --md-default-bg-color: #1e1610;        /* page background */
  --md-default-fg-color: #e8dcc8;        /* body text */
  --md-primary-fg-color: #c88830;        /* header, nav highlights */
  --md-accent-fg-color: #f0a820;         /* links, active elements */
  --md-code-bg-color: #16100c;           /* code block background */
  --md-code-fg-color: #e0d4c0;           /* code text */
  --md-typeset-a-color: #e0b040;         /* link color */
  --md-footer-bg-color: #140e0a;         /* footer background */
}
```

Key selectors for deeper customization:
- `.md-header` -- top bar
- `.md-sidebar` -- left nav panel
- `.md-typeset h1/h2/h3` -- headings
- `.md-typeset table:not([class]) th` -- table headers
- `.md-typeset .admonition` -- callout boxes
- `.md-typeset pre` -- code blocks
- `.md-typeset :not(pre) > code` -- inline code

## GitHub Actions auto-deploy

```yaml
name: docs
on:
  workflow_dispatch:
  push:
    branches: [master]
    paths:
      - 'docs/**'
      - 'mkdocs.yml'

permissions:
  contents: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - run: pip install mkdocs-material
      - run: mkdocs gh-deploy --force
```

Enable GitHub Pages: Settings > Pages > source: `gh-pages` branch.

## API reference doc style

For REST API docs, use per-endpoint sections:

```markdown
### POST /api/endpoint

Description of what it does.

**CLI:** `python tool.py method_name param:value`

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | int | yes | Entity ID |

#### Response (success)

```json
{"id": 123, "name": "Example"}
```

#### Response (error)

```json
{"error": "not found", "id": 999}
```
```

## Rules

- ALWAYS `mkdocs build` before committing to catch warnings
- ALWAYS verify CLI examples match actual Python method parameter names (snake_case), not HTTP body keys (camelCase)
- NEVER add light mode -- dark only unless user requests it
- Keep nav flat -- no nested sections unless 10+ pages
- Use admonitions for warnings, tips, bugs -- not bold text or blockquotes
- Use collapsible blocks for long JSON responses to keep pages scannable
- Use content tabs for CLI vs HTTP examples
- `site/` goes in .gitignore -- never commit build output
- Test locally with `mkdocs serve` before pushing
