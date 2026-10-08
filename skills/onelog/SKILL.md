---
name: onelog
description: "onelog, the operator's local notebook: personal knowledge base, project tracker, and daily log, one YAML file per page. Use when the operator asks to note, log, look up, or track anything, when recording what was done today, or when migrating OneNote pages."
version: "2.0"
---
# onelog

The operator's notebook, replacing OneNote. Local files only. The notebook is
not in git; the onelog app's code is (github.com/abix-/onelog, at
C:\code\onelog). Its backup is `tools/Backup-Notebook.ps1` in that repo:
7-Zip, encrypted, kept on the Synology and uploaded to Google Drive. Its
design is `docs/backup.md` in that repo; run it before any change to the
notebook.
Root: `C:\onelog`. The `ONELOG` environment variable overrides it, but
nothing needs it set.

The page format is `docs/schema.md` in the app repo. That document is the
authority; this skill is how to work with it. Read it before writing a page
by hand.

## Layout

Mirrors OneNote: notebook, sections, pages.

```
<root>/
  <section>/                 named by subject, chosen by the operator
    section.yaml             optional: the section's page order
  puzzles/                   every troubleshoot page, and anything with no other home
    <page>.yaml              a page with no files is a plain file
    <page>/                  a page with files becomes a folder
      <page>.yaml            the page, same name as its folder
      <any files>            screenshots, logs, configs, artifacts for this page
  daily/
    YYYY/
      YYYY-MM-DD.yaml        one page per day, one folder per year
      <any files>            that year's daily screenshots and files
  archive/<section>/<page>/  old sections (OneNote section group "Archive")
  templates/<type>.yaml      one blank page per type
  private/                   NEVER read, list, or search anything in here
```

A page's pictures and files always live in the page's own folder. There is no
shared files folder.

## A page

A page is a canvas of boxes; each box holds paragraphs, tables, images and
files, and every item carries when it was added and last edited. The top of a
daily page:

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
    width: 600.0
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
| project | folder | the project's folder on disk (repo root, or a mod's folder inside modforge); its `docs/todo.md` and `docs/changelog.md` show on the page |
| project | binaries | the programs the project builds and runs (name, exe, args, folder, build, start_on_login), for the app's task manager; see `docs/schema.md` |
| topic | (none) | |
| troubleshoot | status | open, fixed, gave-up |
| troubleshoot | project | project page name, optional |
| list | (none) | the page holds one table |

## Writing a page by hand

The app reads pages strictly: an unknown field, a missing field, a string
without quotes where one is needed, or a table row with the wrong number of
cells stops the page from opening, with the line named. So:

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
   checkboxes on its page; they go in its `docs/todo.md` (see Project page).
7. An item indented under another goes in that item's `children`.

## Rules

1. Names are lowercase with hyphens. No spaces.
2. A page with no files is a plain file: `<section>/<page>.yaml`. When it gets
   its first file, move it into a folder: `<section>/<page>/<page>.yaml` plus
   the file. Daily pages always stay plain files: `daily/YYYY/YYYY-MM-DD.yaml`,
   with any daily files in the year folder.
3. Page names are unique across the whole notebook.
4. Dated pages (daily, troubleshoot) start with `YYYY-MM-DD`.
5. Work is project based. Every project gets a `type: "project"` page in the
   `projects` section: `projects/<name>.yaml`. Every GitHub repo is a project,
   named after the repo lowercased (list them with
   `gh repo list --limit 1000`). A project inside another repo (such as a game
   mod inside a mod framework) gets its own page named after its folder, with
   `parent` set to the repo's page.
6. Links between pages are `[[page-name]]` in paragraph text. A page's own
   pictures and files are `image` and `file` items naming a file in the page's
   folder.
7. Status fields take only the values listed above.
8. A list page holds one table; its column names never change once set.
9. Each fact lives on one page. The daily page links to it, never copies it.
10. A new type means updating this skill, `docs/schema.md` and adding a
    template first.
11. New pages start from `templates/<type>.yaml`, with every time in it set to
    now. Set `updated` on every edit.

## Daily page

Records what was done that day, short bullets. All work is part of a project:
every `h2` heading's text starts with its project link, `[[project]]` or
`[[project]]: subject`, and the bullets under it are `list: "bullet"`
paragraphs. A bullet that belongs to another project goes under that
project's heading. If no project fits, create the project page first. Add
every project with a heading to `projects`. Create the day's page from the
template if missing.

## Project page

Headings Goal, Current state, Decisions. Current state says what is true now
and the dates it was worked on; it is rewritten, not appended to.

A project's todo and changelog are NOT on its page. They live in the
project's folder as `docs/todo.md` and `docs/changelog.md`, kept in the
`todo` and `changelog` skills' standards by the Claude doing the work. The
page names that folder in `folder`; the onelog app shows both files beside
the page and gathers every project's todo rows in its Tasks view. onelog
never writes them, and nothing from them is copied into the notebook. Every
modforge mod has its own folder, todo and changelog.

The daily page still records what was done each day under the project's
heading; find a project's days by searching them for `[[<name>]]`.

## Finding things

- Text: Grep over the root for `*.yaml`, excluding `private/`. Paragraph text
  is on `text:` lines.
- Fields: grep the top-level fields (e.g. every `type: "troubleshoot"` with
  `status: "open"`).

## Privacy

Every file Claude reads is sent to Anthropic. Read only what the task needs.
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
