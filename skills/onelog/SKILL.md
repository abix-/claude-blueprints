---
name: onelog
description: "onelog, the operator's local notebook: personal knowledge base, project tracker, and daily log, its pages in a SQLite database read and written through the onelog app's MCP tools. Use when the operator asks to note, log, look up, or track anything, when recording what was done today, or when migrating OneNote pages."
version: "3.2"
---
# onelog

The operator's notebook, replacing OneNote. Local only. The notebook is not
in git; the onelog app's code is (github.com/abix-/onelog, at
C:\code\onelog). Its backup is `tools/Backup-Notebook.ps1` in that repo:
a consistent copy of the database plus the page folders, 7-Zip, encrypted,
kept on the Synology and uploaded to Google Drive. Its design is
`docs/backup.md` in that repo; run it before any change to the notebook.
Root: `C:\onelog`. The `ONELOG` environment variable overrides it, but
nothing needs it set.

## Where the pages are

Every page is in one SQLite database, `<root>/notebook.sqlite`, kept by the
onelog app (`docs/schema.md` in the app repo, "Storage: SQLite"). Pages are
read and written only through the app's MCP tools (the `onelog` MCP server,
`http://127.0.0.1:31300/mcp`, there in every session while the app is open).
Never read, grep or write the database file. The YAML files that were the
store before 2026-10-09 were imported into it and deleted. If the tools are not
there, onelog is not running: say so and ask the operator to open it.

| To | Tool |
|---|---|
| see what the window shows | `state` |
| list sections, open one (its pages) | `list_sections`, `open_section` |
| list projects (name, status, folder) | `list_projects` |
| read a page's text | `read_page` |
| replace a page's text | `write_page` (the text is checked; a page that does not read is refused) |
| add a paragraph, list item or heading at the end of a page | `append` |
| make a section or a page from its template | `new_section`, `new_page` |
| today's daily page, made from the template when missing | `today` |
| search every page | `search` |
| a project's features table | `features`, `set_feature`, `remove_feature` |
| a project's todo, or every project's | `todo`, `todo_add`, `todo_change`, `todo_check`, `todo_finish` (the `todo` skill) |
| a project's rank | `set_rank` |
| open a page in the window | `open_page` |

A page is named by its section and its name: `read_page` with section
`projects`, page `onelog`; a daily page is its date, `2026-10-09`.

A page's pictures and files stay on disk, in the page's folder under the
root: `<section>/<page>/` for a page with files, `daily/YYYY/` for daily
pages. `private/` is not in the database: NEVER read, list, or search
anything in it.

## A page

The page format is `docs/schema.md` in the app repo. That document is the
authority; this skill is how to work with it. Read it before writing a page
with `write_page`.

A page is a canvas of boxes; each box holds paragraphs, tables, images and
files, and every item carries when it was added and last edited. The top of a
daily page, as `read_page` gives it:

```yaml
schema: 1
type: "daily"
title: "2026-10-07"
created: "2026-10-07T04:00:00Z"
updated: "2026-10-07T04:00:00Z"
projects: ["onelog", "chromium-extensions"]
boxes:
  - x: 0.0
    y: 0.0
    added: "2026-10-07T04:00:00Z"
    edited: "2026-10-07T04:00:00Z"
    content:
      - text: "[[onelog]]"
        added: "2026-10-07T04:00:00Z"
        edited: "2026-10-07T04:00:00Z"
        style: "h2"
      - text: "Built the backup."
        added: "2026-10-07T20:14:00Z"
        edited: "2026-10-07T20:14:00Z"
        list: "bullet"
```

A box with no `width` fills the page's width.

Page types and their extra fields:

| type | field | values |
|---|---|---|
| daily | projects | list of project page names touched that day |
| project | status | active, paused, done, dropped |
| project | repo | GitHub URL, optional |
| project | parent | parent project page name, when the project lives inside another repo |
| project | fork | true, false (GitHub repos only) |
| project | visibility | public, private (GitHub repos only) |
| project | language | GitHub primary language (GitHub repos only) |
| project | folder | the project's folder on disk (repo root, or a mod's folder inside modforge); its `docs/changelog.md` shows on the page |
| project | rank | whole number from 1, the projects to work on first; optional |
| project | todo | the project's todo rows (`priority`, `system`, `todo`, `done_when`, and `done` with `done_at` once checked); kept with the todo tools, see Project page |
| project | binaries | the programs the project builds and runs (name, exe, args, folder, build, start_on_login), for the app's task manager; see `docs/schema.md` |
| project | features | how close the project is to done: each feature in a few words with a `score` from 0 to 10 and an optional `note` on where it stands; see Project page |
| topic | (none) | |
| troubleshoot | status | open, fixed, gave-up |
| troubleshoot | project | project page name, optional |
| list | (none) | the page holds one table |

## Writing a page

Prefer `append` for adding to a page and the feature tools for the features
table. For any other change, `read_page`, change the text, `write_page` it
whole. The app reads pages strictly: an unknown field, a missing field, a
string without quotes where one is needed, or a table row with the wrong
number of cells is refused, with the line named. So:

1. Every string is quoted with double quotes.
2. Fields go in the order `docs/schema.md` lists them; copy the order of the
   items around the one you add.
3. Times are UTC to the second with a `Z`: `date -u +%Y-%m-%dT%H:%M:%SZ`.
   Never invent a time.
4. A new item gets `added` and `edited` set to now. An edit sets `edited` on
   the item and on every box, table, row and cell it sits in, and the page's
   `updated`. `added` never changes.
5. Paragraph text uses the inline subset: `**bold**`, `*italic*`,
   `~~strike~~`, `` `code` ``, `[label](url)`, `[[page-name]]`. A literal
   `*`, `[`, `]`, `` ` `` or `\` is escaped with `\`, and inside the quoted
   YAML string every `\` is doubled: a path `C:\onelog` is written
   `"C:\\\\onelog"`.
6. A heading is a paragraph with `style: "h1"` to `"h6"`. A bullet is
   `list: "bullet"`, a numbered item `list: "number"`, a checkbox
   `list: "task"` with `done: false` and `task_added`. Checking one sets
   `done: true`, `done_at` and `edited`. A project's work items are not
   checkboxes on its page; they are its todo rows (see Project page).
7. An item indented under another goes in that item's `children`.

## Rules

1. Names are lowercase with hyphens. No spaces.
2. Page names are unique across the whole notebook.
3. Dated pages (daily, troubleshoot) start with `YYYY-MM-DD`.
4. Work is project based. Every project gets a `type: "project"` page in the
   `projects` section (`new_page` in `projects`). Every GitHub repo is a
   project, named after the repo lowercased (list them with
   `gh repo list --limit 1000`). A project inside another repo (such as a
   game mod inside a mod framework) gets its own page named after its
   folder, with `parent` set to the repo's page.
5. Links between pages are `[[page-name]]` in paragraph text. A page's own
   pictures and files are `image` and `file` items naming a file in the
   page's folder.
6. Status fields take only the values listed above.
7. A list page holds one table; its column names never change once set.
8. Each fact lives on one page. The daily page links to it, never copies it.
9. A new type means updating this skill, `docs/schema.md` and the app first.
10. New pages start from the `templates` section's page for their type
    (`new_page` and `today` do this).

## Daily page

Records what was done that day, short bullets. All work is part of a project:
every `h2` heading's text starts with its project link, `[[project]]` or
`[[project]]: subject`, and the bullets under it are `list: "bullet"`
paragraphs (`append` with `heading: 2`, then `append` with `list: "bullet"`).
A bullet that belongs to another project goes under that project's heading.
If no project fits, create the project page first. Add every project with a
heading to `projects` (`read_page`, change the field, `write_page`). `today`
makes the day's page from the template when missing.

## Project page

Headings Goal, Current state, Decisions. Current state says what is true now
and the dates it was worked on; it is rewritten, not appended to.

Every project keeps a features table in its page's `features` field: each
feature the project needs, in a few words, with a score from 0 to 10 for how
done it is and a short note on where it stands. The project's overall score
is the mean, and the app shows the table and the overall score under the
title, so how close a project is to done shows at a glance. A score is what
the evidence shows, not what was hoped: a feature that is built but not
proven is not 10. Set and change rows with the app's `set_feature` and
`remove_feature` tools (`features` reads them); update the table whenever
the project's work changes it.

A project's todo is on its page, in the `todo` field, kept through the todo
tools in the `todo` skill's standard. Its changelog stays in the repo as
`docs/changelog.md`, in the `changelog` skill's standard, since it records
the work done there; the page names the repo in `folder` and the app shows
the changelog on the page. Each row has a box. The Claude agent working
a row runs its Done when test, checks it (`todo_check`), takes it off
(`todo_finish`, which refuses an unchecked row), writes the changelog row in
the repo, then commits and pushes; a row the operator checked by hand waits
checked for the next agent. The page is one column: features, the page's
boxes, the todo, then the changelog, each folding away. The Tasks view
gathers every project's rows, by `rank` then priority. Every modforge mod has
its own page and so its own todo; the changelog is modforge's. A project
whose repo still has a `docs/todo.md` has not been moved yet
(`onelog-import todos` moves it).

The daily page still records what was done each day under the project's
heading; find a project's days with `search` for `[[<name>]]`.

## Privacy

Every page Claude reads is sent to Anthropic. Read only what the task needs.
Never open `private/`. Secrets belong in a password manager, not here.

## OneNote migration

Pages are imported from OneNote's own page XML, never from the `1note`
skill's text output (which drops times, formatting and positions). Export
with `tools/export-onenote.ps1` (Windows PowerShell 5.1, not pwsh), then
`onelog-import onenote <export folder> <section folder>`, which converts,
writes, reads back and proves every page, and never overwrites one. It is an
import, not a reword: every paragraph, cell and time is kept exactly as
OneNote has it, one onelog page per OneNote page. Never summarize, merge,
reorder, fix typos, or add headings. Index pages named `-Something` become
`type: "list"`. One-page sections are migrated only if their page has
content. The mapping is in `docs/schema.md`.
