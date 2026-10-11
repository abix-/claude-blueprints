---
name: onelog
description: "onelog, the operator's local notebook: personal knowledge base, project tracker, and daily log, its pages in a SQLite database read and written through the onelog app's MCP tools. How all work is recorded and handled: a project's todo rows, the repo's docs/changelog.md, the daily page, cleaning up a todo, finishing work. Use when the operator asks to note, log, look up, or track anything, when starting or finishing work, when recording what was done today, for /todo, /changelog, /done or a cleanup of done todo items, or when migrating OneNote pages."
user_invocable: true
version: "3.4"
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
| a project's todo, or every project's | `todo`, `todo_add`, `todo_change`, `todo_check`, `todo_finish` (see Todo) |
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

## When onelog is updated

onelog is updated when work starts and again when it is done, so it shows
the work in progress, not only what is finished. Never only at the end of a
batch.

- **Starting work.** Before the first change, the work is a row on its
  project's todo (`todo_add` if it is not there yet, in the Todo rules
  below), and today's daily page has a bullet under the project's
  heading saying what is starting, beginning `In progress:`. Work across
  several projects gets each project's heading and bullet as that project's
  work starts.
- **Finishing work.** When the row's Done when test passes: `todo_check`,
  `todo_finish`, the changelog row in the repo, commit and push; then the
  daily bullet is rewritten (`read_page`, `write_page`) to say what was done,
  with its commit, and loses `In progress:`.
- **Stopped part way.** The row stays open and the bullet says where the
  work stopped and what is left.
- **Picking up after a break or compaction.** Read today's daily page and
  the project's open rows first, and carry on from its `In progress:`
  bullet.
- Work goes on the daily page of the day its commits were made.

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
tools by the Todo rules below. Its changelog stays in the repo as
`docs/changelog.md`, by the Changelog rules below, since it records the work
done there; the page names the repo in `folder` and the app shows the
changelog on the page. One writer for each: onelog keeps the todo, the repo
keeps the changelog. Each row has a box. The Claude agent working
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

## Todo

Every project's todo lives on its project page in onelog, not in the repo.
Read and change it only through onelog's MCP tools; never edit the notebook
any other way. If the tools are not there, onelog is not running: say so and
ask the operator to open it.

| To | Tool |
|---|---|
| a project's rows, lowest priority first | `todo` with `project` |
| every project's rows, by project rank then priority | `todo` with no project |
| add a row | `todo_add`: `project`, `priority`, `system`, `todo`, `done_when` |
| change a row | `todo_change`: `project`, `todo` (the row as it is), then any of `priority`, `system`, `new_todo`, `done_when` |
| check a row once its Done when test passes | `todo_check`: `project`, `todo` (`done: false` unchecks it) |
| take a checked row off | `todo_finish`: `project`, `todo` (refuses an unchecked row) |
| order the projects | `set_rank`: `project`, `rank` (1 first) |

The project is its project page's name (`list_projects`): the repo name
lowercased, or a modforge mod's folder name. A row is named by its Todo text,
which is unique in the project.

### When asked to show or change a todo (`/todo`)

1. Find the project: the page whose `folder` is the current repo or mod
   folder (`list_projects`).
2. Read its rows with `todo`.
3. If the operator gave an argument (e.g. `/todo add fix the harmony
   bridge`), apply it as described in Todo operations. With no argument, show
   the rows and ask what to change.

A project whose todo is still in its repo's `docs/todo.md` (the `todo` tool
with no project names them) has not been moved yet: its todo stays in that
file, under the rules below, until `onelog-import todos` moves it. Never keep
rows in both places. A `docs/todo.md` that strays from these rules is
brought to them first (see Cleaning up a todo).

### Todo rules

These rules govern every project's todo. Follow them exactly.

- Every row has exactly four parts: `Priority`, `System`, `Todo`, and
  `Done when`.
- Put a number from `1` through `100` in the Priority column. Lower numbers
  come first. Use the full range so the table shows real differences between
  tasks.
- Priority measures how quickly completing the row gets the project closer to
  its next shipped state. It does not measure alarm, age, code size, test
  status, or how easy the work looks.
- Priority `1` is any concrete task with the greatest direct effect on
  shipping. Multiple open rows may have priority `1` when each has that
  effect. Priority `100` is valid work with the least direct effect.
- Reassign priorities when the goal or todo changes. The `todo` tool lists
  rows from lowest number to highest. The first row is next unless the
  operator says otherwise.
- Put the actual system being changed in the System column, using its name
  from the codebase (crate, module, package, file). Do not use a generic
  category or a made-up name.
- The Todo is the instruction alone, with no checkbox in its text: `Do the
  thing`. The row's box shows its state: unchecked is open, checked is done
  and proven, waiting to be moved to the changelog.
- The todo is the ordered work plan. Before changing source code, split the
  work into the concrete rows required to finish it.
- Each row is exactly one action that can be started, completed, and proved in
  one session.
- Each row must be independently executable as written. Completing it must
  leave production source compiling without temporarily adding a second path,
  partially implementing another row, or requiring an unlisted follow-up edit.
- Write the Todo cell as a direct instruction to change one behavior. Write
  the Done when cell as the one exact result that proves that instruction is
  finished.
- Use plain terms. Do not use vague tasks, invented language, unexplained code
  names, or design essays.
- Done when names the exact observable result or proof that closes the row.
  Partial work stays on the list.
- A newly found issue gets a new prioritized row. Never enlarge the active
  row or create a nested or duplicate checklist.
- `Update the todo` means change affected rows only. Do not add status
  paragraphs, percentages, summaries, diaries, or evidence dumps.
- The Claude agent working a row finishes it, in this order: do the work; run
  the Done when test and see it pass; `todo_check` the row; `todo_finish`
  takes it off; record it as a row in the repo's `docs/changelog.md` (see
  Changelog); commit the code and the changelog row together and push. A
  project with no repo records it on today's daily page instead
  (`todo_finish` says which).
- A row the operator checked by hand stays checked on the page. The next
  agent working that project moves it to the changelog the same way.

### Todo operations

- `/todo` with no args: show the project's rows.
- `/todo add <description>`: `todo_add`. Ask for priority and system if not
  obvious from context.
- `/todo done <row>`: before moving a completed row, read the project's design
  docs (`docs/design.md`, `docs/authority.md`, `docs/construction.md`, or
  whatever the project uses) and verify the row's "Done when" claim aligns
  with the documented design. Check for sabotage: new competing models,
  undocumented abstractions, or state changes beyond what the row describes.
  Report the alignment verdict and wait for operator approval. On approval,
  `todo_check` and `todo_finish` the row and add the changelog row. Process
  one row at a time.
- `/todo reprioritize`: reassign priorities based on current state with
  `todo_change`.

### Multi-crate workspaces

Each modforge mod has its own project page (named after its folder) and so
its own todo; cross-cutting work goes on the `modforge` page. The changelog
is always repo-wide (`docs/changelog.md`), never per-crate.

The todo needs no commit: onelog saves it. Commit the changelog when a row
was recorded there, and push.

## Changelog

Each repo records its completed work in `docs/changelog.md`. If the file does
not exist, create it with the header and rules below.

### When asked to record or show the changelog (`/changelog`)

1. Read `docs/changelog.md`.
2. If the operator gave an argument (e.g. `/changelog add fixed the harmony
   bridge`), add a row to today's table as described below.
3. With no argument, show today's table (or the most recent day) and ask what
   to record.

### File format

The file starts with a short header block, then daily tables newest first. The
header must include:

```markdown
# <project name> changelog

## Changelog rules

<the rules below, verbatim>
```

When creating a new `docs/changelog.md`, copy the rules section below into it
verbatim so every project carries the same instructions.

### Changelog rules

These rules govern `docs/changelog.md`. Follow them exactly.

- Each day has exactly one `## YYYY-MM-DD` heading and exactly one table below
  it.
- Every daily table has exactly three columns: `System`, `Item`, and
  `Done when`.
- Start every Item with `[x]`. No preceding hyphen. Changelog rows are
  completed work only.
- Never combine completed items into one changelog row. Never replace them with
  a summary paragraph, grouped bullet, status paragraph, or count.
- When recording a completed todo row, copy its System into `System`, its
  completed instruction into `Item`, and its completion proof into `Done when`.
- Verify the exact result before adding the changelog row.
- Leave unfinished work in the todo. Do not add partial work to the changelog.
- Put new days above older days. Add rows to the existing table when that date
  already exists. Never create a second heading or second table for the same
  day.
- Use plain terms. Do not use vague descriptions, invented language, or design
  essays.

### Changelog operations

- `/changelog` with no args: show the most recent day's table.
- `/changelog add <description>`: add a row to today's table. Ask for system
  if not obvious from context. Create today's heading if it does not exist.
- `/changelog show <date>`: show a specific day's entries.

After any change, commit `docs/changelog.md` and push.

## Cleaning up a todo

For a todo or backlog that has accumulated completed items that need to move
into authoritative docs, or when asked to clean up, archive, or relocate
"done" entries: a repo's `docs/todo.md` before `onelog-import todos` moves it,
and the same rules for what may sit in a onelog todo row.

Completed items pile up in a todo file and must move to the authoritative doc
that owns that topic. The danger is losing data: deleting from the source
before confirming the destination really holds every line.

**Core principle: MOVE, then VERIFY, then DELETE. Three separate passes, in
that order, never fused.** The source is the backup until verification
passes. Deleting before verifying is the one mistake that destroys work
permanently.

### Documentation layout standard (repo-wide, LOCKED)

Written for the jbot (lotj) and factoriobot repos, whose docs follow it.

Every authoritative doc MUST begin in this exact order:

1. **Authority statement** (the explicit `Authoritative for:` line near top).
2. `## Table of contents`.
3. **Definition of 10/10** (explicit section defining what 10/10 means for
   that doc's domain).
4. **Ratings table** (authority-style scored table).

No alternate ordering is allowed. Do not insert other top sections between
these blocks.

This is NOT a generic "Doc/Status/Top action" scaffold. Every doc must have
domain-specific scored rows with evidence and concrete next actions.

Canonical ratings table design (must match `docs/authority.md` style):

| # | Item | Score | Evidence snapshot | Top action |
|---|---|---|---|---|
| 1 | <domain item> | X/10 | <why this score, current evidence> | <next concrete move> |

Design consistency rules (non-optional):

- Use this exact five-column order: `#`, `Item`, `Score`, `Evidence snapshot`,
  `Top action`.
- `Score` values are always rendered as `X/10` (never plain `X`, never `%`,
  never badges).
- Keep the ratings table at the top block immediately after the 10/10
  definition.
- Keep separator style exactly: `|---|---|---|---|---|`.
- Use concise, evidence-first phrasing in `Evidence snapshot`; use one
  concrete next move in `Top action`.
- If a doc has one whole-system row, still keep this same table shape.

Required scoring doctrine:

- **10/10 has one strict definition (LOCKED):** the system can run for
  **365 continuous days** for **10 concurrent players** with **no Rust
  updates required**. The only allowed changes are **per-player YAML
  configuration/preferences**. Every relevant path is handled correctly, and
  there are **ZERO known bugs**.
- Anything short of that bar is below 10/10.
- **1/10** means effectively unowned / nonfunctional in intended scope.
- Scores in between must be justified by observable evidence in that doc's
  domain (tests, audits, known regressions, rollout status, operator
  outcomes).
- Low scores are encouraged when uncertain; do not inflate.

When cleanup relocates or normalizes docs, preserve or upgrade targets to this
exact locked shape and order. Do not invent per-file variants, and do not bury
ratings in body prose.

When writing each doc's `## Definition of 10/10`, use this strict definition
verbatim in domain-adapted wording (same meaning, no weaker phrasing).

### What belongs in a todo (the one rule)

A todo is a list of CONCRETE TASKS and nothing else. Exactly two things belong
in it:

1. **Concrete tasks.** "Do X": something a person can pick up and check off
   (build / add / fix / port / wire / delete X).
2. **Links to the CONTEXT** for those tasks: a pointer to the authoritative
   doc that holds the design, status, or background.

EVERYTHING else is DATA, and ALL data gets folded into whatever authoritative
doc owns it, then replaced in the todo by a task line plus a link. Data
includes: design / doctrine / rationale, status / root-cause / "current
state", tables, dashboards, coverage snapshots, captured logs, code-shape
sketches, examples, verbatim quotes, narrative. None of it lives in the todo.

This is the ONE rule the rest of this section implements. Design
documentation goes to its subject doc; status goes to `status.md`; a doctrine
block with a thin actionable core leaves the doctrine in the doc and keeps
the task. Same move every time: fold the data into the authoritative doc,
leave the task plus a link.

**Data that already has an authoritative home is not re-relocated; it is
DROPPED and linked.** A table hand-copied from a machine source (e.g. a
coverage dashboard duplicating `playbook_coverage.yaml` and its printer) is a
stale-prone duplicate. Delete it from the todo and replace it with a one-line
pointer to the source of truth. Do not relocate the snapshot anywhere; the
registry already is the record.

### The three passes

Run them as distinct steps. Do not delete anything in the same pass that
moves it.

**1. MOVE (copy into the authoritative doc)**
- Identify each completed block, ONE at a time. `## Done (date)` headings are
  the obvious case. A `## P0` / doctrine block counts as completed only when
  its named artifact (module/file, predicate, verb, field) actually EXISTS and
  is WIRED in the code: grep the source (e.g. `jbot/src`) for the symbol and
  confirm real callers. Judge done-ness against the code, never by reading the
  block's prose.
- For each block, choose the authoritative target doc and a stable anchor
  slug.
- Append the **full body, verbatim** to the target under a clearly marked
  section (e.g. `## Relocated from todo.md`, block headed `### <anchor>`). No
  breadcrumb needed; the anchored section IS the record.
- Dedupe by anchor so re-runs are idempotent (skip if the anchor already
  exists).
- **Do not touch the source todo in this pass.**

**2. VERIFY (confirm nothing was lost)**

**This is the mandatory standard for every cleanup. Two independent checks
per block; both must pass before that block is eligible to delete.**

- **Source of truth for the original body:** if the working tree already
  holds stubs or partial content (a prior MOVE pass ran), you CANNOT
  reconstruct the original from a stub. Read the full original body from git:
  `git show HEAD:<source-file>`. The committed version is the source of truth,
  not the working tree.
- **Check A: line presence.** Every non-empty line of the original body must
  appear in the target doc. Mechanical, not eyeballed (see the script below).
  Report the count of missing lines; nonzero means NOT safe.
- **Check B: anchored landing exists.** The block must live under a real
  `## Relocated from todo.md` section at a concrete `### <anchor>`. Confirm
  the anchor heading physically exists in the target (e.g. grep
  `^### <anchor>`) and report its `file:line`. "Lines appear somewhere in the
  file" is not enough; the section must exist.
- Confirm the target docs are saved (and committed, if the workflow commits
  before delete).
- Build the explicit delete-safe list: only blocks that pass BOTH checks.
  Print a `N delete-safe, M NOT safe` summary.
- Any block with no target mapping, a failing line check, or a missing anchor
  stays in the source and gets reported. Never delete it.

**3. DELETE (remove from the source)**
- Remove only the verified blocks from the todo.
- Re-read the todo afterward: confirm exactly the intended blocks are gone and
  surrounding structure is intact.

### Partial blocks (shipped core + open follow-ups)

A doctrine block is often mostly shipped with a few open follow-ups (e.g. a P0
block whose "out of scope this round" list is still real work). Do not keep
the whole block, and do not lose the follow-ups:

- **MOVE** the full shipped doctrine body to the authoritative doc. The design
  is now reality; the doc is its record.
- In the **DELETE** pass, replace the source block with a small residual block
  holding ONLY the still-open follow-ups, plus a one-line pointer to the
  relocated record. This is a trim, not a full delete; the open items stay
  tracked in the todo.
- VERIFY still runs on the moved body (both checks) before the trim.

### Design documentation is not a todo item

A todo file should hold a concrete, actionable list: things that can be DONE
and checked off. Much of what accumulates in a long-lived todo is not that. It
is DESIGN DOCUMENTATION: doctrine, locked decisions, rationale ("why P0"),
operator-verbatim context, code-shape sketches, architecture. That belongs in
the owning design / subject doc, not the todo, whether or not the related work
is finished.

Classify each block by what it mostly IS, not by its done/open status:

- **Actionable item.** "Do X": a checkbox, a concrete deliverable, a named gap
  to close. Stays in the todo.
- **Design documentation.** Doctrine, rationale, decisions, "why", verbatim
  quotes, code shape. Belongs in the design doc.

Most long doctrine blocks are a thin actionable core wrapped in thick design
prose. Handle them like a partial block, but split on TYPE rather than on
shipped-vs-open:

- **MOVE** the design documentation (doctrine, rationale, decisions,
  code-shape) to the owning design doc under `### <anchor>`. It is the
  authoritative record of the design, done or not.
- VERIFY (both checks) on the moved prose.
- **DELETE** down to a residual that is ONLY the concrete actionable items
  still to do, plus a one-line pointer to the design doc. If nothing
  actionable remains, the block leaves the todo entirely.

The test for any residual line: can someone DO it and check it off? If it
explains or justifies, it is documentation, so move it. If it is "build / add
/ fix / port X", it is a todo, so keep it.

### Status and root-cause analysis go in ONE living status.md

A todo holds actionable items. It is NOT the place for "current state"
narrative: root-cause analysis, investigation write-ups, "honest status",
"what shipped / what didn't", "where we are now". That is STATUS, and status
belongs in ONE living `status.md` that explains the current state and what we
are doing.

`status.md` is handled DIFFERENTLY from every other relocation in this
section, and the difference is the whole point:

- **Every other target is append-only** (the doctrine lands under a new
  `### <anchor>` and accumulates; old records are kept). **status.md is the
  opposite: ONE file, UPDATED in place.** It is a single snapshot of current
  reality, not a log. You REWRITE the relevant section so it reflects what is
  true now, and you REPLACE stale status rather than appending a new dated
  entry beside it.
- So status does NOT go through the append-under-anchor MOVE path or the
  generic tool. You edit `status.md` directly: fold the current-state facts
  in, overwrite any line that is now stale, then remove the status/root-cause
  prose from the todo.
- If `status.md` does not exist yet, create it once (current state + what we
  are doing now); thereafter only update it.
- VERIFY for status is "is the current fact represented in status.md?", not a
  verbatim line-check (you are condensing and replacing, not copying). Confirm
  the fact is there, then delete from the todo.

A root-cause write-up that is still being chased is the tricky case: the
FINDINGS and current state go in status.md; the remaining concrete fix steps
stay in the todo as actionable items. Same split as design documentation,
just routed to status.md instead of a subject doc.

**Every subsystem in status.md carries a brutal X/10 maturity rating.** 10/10
means complete: it needs NO updates for 10 years AND is perfect, free of ALL
bugs. Anything actively churning, missing features, or carrying a single open
bug is well below 10. Rate honestly and low. The rating exists to show where
the real risk is, not to flatter. Put a one-line legend at the top of
status.md and the rating on each subsystem (e.g. `## Pump transport -- 3/10`),
and re-rate when you update that subsystem's state.

**status.md is the status of the ENTIRE system, not just the parts with todo
entries.** Cover every runtime component and subsystem (for jbot: the pump /
transport, the brain and each of its subsystems, the Discord bot). A component
with zero open todo items still gets a section and a rating, so the doc is a
complete health board at a glance, never a partial view skewed toward
whatever happened to be in the backlog.

### Verification check (the standard, do this, don't eyeball)

```python
import re, subprocess

# Original body comes from the committed source, not the stubbed working tree.
def head_body(source_path: str) -> str:
    return subprocess.run(["git", "show", f"HEAD:{source_path}"],
                          capture_output=True, text=True, check=True).stdout

# Check A: every meaningful line of the original body landed in the target.
def lines_present(body: str, target_text: str) -> list[str]:
    return [ln for ln in body.splitlines()
            if ln.strip() and ln.strip() not in target_text]  # empty == pass

# Check B: the anchored landing section physically exists in the target.
def anchor_exists(anchor: str, target_text: str) -> bool:
    return re.search(rf"^### {re.escape(anchor)}\b", target_text, re.M) is not None
```

A block is delete-safe ONLY when `lines_present(...)` is empty AND
`anchor_exists(...)` is True. Report a per-block `OK / MISSING / NO ANCHOR`
line and a `N delete-safe, M NOT safe` total. This logic belongs in the ONE
generic cleanup tool (see below), not an ad-hoc one-liner per cleanup.

### Recurring cleanups: one keyed registry, not ad-hoc targets

When the same todo gets trimmed repeatedly (a living backlog), do not
re-specify each block's target doc, anchor, and residual fresh every run.
Promote them into ONE keyed registry that all three pass-scripts read, so
MOVE, VERIFY, and DELETE can never disagree about where a block went or what
to leave behind.

Shape (one entry per block, keyed by a short slug):

```python
BLOCKS = {
    "lifecycle": {
        "needle":   "framework owns the playbook lifecycle",   # locates the block in the source
        "doc":      "data-flow-and-authority.md",              # authoritative target
        "anchor":   "framework-lifecycle-doctrine-2026-05-30", # stable landing slug
        "note":     "Shipped 2026-05-30: ... Authoritative design record.",
        "residual": "## OPEN ... only the still-open follow-ups",  # "" means full delete
    },
    # ... one entry per completed block
}
```

- **MOVE** extracts by `needle`, appends the body to `doc` under
  `### {anchor}` (dedupe by anchor, idempotent).
- **VERIFY** checks every body line is present in `doc` and that
  `### {anchor}` physically exists.
- **DELETE** replaces the source block with `residual` (the partial-block trim
  from the section above, pre-declared in the registry so it is reviewed up
  front, not improvised), or removes the block entirely when `residual` is
  `""`.

One registry means each block's target and anchor are declared once and
reused by all three passes; a per-run ad-hoc target list is exactly how MOVE
and VERIFY drift apart. Invoke per block by key so each block is still handled
ONE at a time, satisfying the one-block-per-pass rule.

**ONE generic tool, not a script per cleanup.** All of this (block
relocation, partial-block trim, completed-`[x]`-item relocation) is the SAME
move/verify/delete shape over two selectors: a `## ` heading block, or the
`- [x]` lines inside a section. Do not write a fresh `relocate_*.py` /
`verify_*.py` / `delete_*.py` per cleanup; that breeds a pile of
near-duplicates. Put it in ONE generic tool keyed by job, e.g.
`scripts/cleanup.py <key> <move|verify|delete|run>` with
`kind: "block" | "checked"`. Adding a cleanup is one registry entry, never a
new script. Past jobs are recorded by their `### <anchor>` sections in the
target docs, so the registry only needs current jobs.

### Common mistakes

- **Fusing move and delete in one edit.** A crash or a bad replace then loses
  the block with no copy anywhere. Keep the passes separate.
- **Eyeballing instead of line-checking.** Truncated or reflowed paste looks
  fine at a glance. Run the line check.
- **Deleting a block that had no target mapping.** No mapping means it was
  never relocated. Leave it and report it.
- **Anchor collision overwriting an earlier relocation.** Dedupe by anchor;
  never silently replace.
- **Careless text replacement that shifts offsets.** Match on the full block
  text, not line numbers.
- **Treating design documentation as a todo.** Doctrine, rationale,
  decisions, and "why" belong in the design doc; the todo keeps only concrete
  actionable items. Leaving a 200-line doctrine block in the todo because
  "some work is still open" keeps documentation in the wrong file.
- **Treating status / root-cause analysis as a todo.** "Honest status", "what
  shipped", investigation write-ups, and "current state" notes are status,
  not actionable items. They go in `status.md`.
- **Appending status instead of updating it.** Status is ONE current
  snapshot. Appending dated status entries (or relocating it under a
  `### <anchor>` like a doctrine) recreates the log-in-the-wrong-place
  problem. Update `status.md` in place and replace stale lines.
- **Leaving DATA in the todo.** Tables, dashboards, coverage snapshots,
  captured logs, examples, narrative: none of it is a task. Fold it into the
  authoritative doc (or drop + link if a registry already owns it). The todo
  keeps concrete tasks + context links only.

### Red flags (STOP)

- "I'll just move and delete in the same step."
- "It clearly landed, I don't need to check each line."
- "This block has no obvious home, I'll delete it anyway."
- "The script said OK so it's fine." Verify the destination yourself before
  any delete.
- "This block is still open, so it stays." Open status does not make design
  prose a todo. Move the doctrine to the design doc; keep only the actionable
  items.
- "This is a root-cause write-up / honest status / current state." That is
  status, not a todo. Fold it into `status.md` (update in place, replace stale
  lines) and remove it from the todo; keep only the remaining fix steps.
- "This table / dashboard / example / capture is useful here." It is DATA,
  not a task. Fold it into the authoritative doc, or drop it and link if a
  registry already owns it. The todo keeps only concrete tasks + context
  links.

All of these mean: do not delete yet. Re-run VERIFY first.

## Finishing work in endless (`/done`)

After completing feature work in endless: update docs, changelog, commit, and
push.

1. **Run clippy**: `k3sc cargo-lock clippy --manifest-path
   /c/code/endless/rust/Cargo.toml 2>&1`. Fix any warnings before proceeding.
2. **Identify changed files**: Run `git diff --name-only` and
   `git diff --cached --name-only` to see what changed.
3. **Read docs/README.md**: Find which architecture doc(s) cover the changed
   systems. Use the System Map and File Map to match changed files to docs.
4. **Update architecture docs**: For each relevant doc in docs/:
   - Read the current doc
   - Read the changed source code
   - Update the doc to match the new code (data flow, components, known
     issues, ratings)
   - If a new system was added, create a new doc and add it to the
     docs/README.md index
5. **Update CHANGELOG.md**: Add entry describing what changed. Follow
   existing format.
6. **Commit and push**: Stage all changed files (source + docs + changelog),
   write a concise lowercase commit message, push immediately.

Rules:
- Don't update docs that aren't affected by the code changes
- Don't add content to docs that isn't in the code. Docs reflect reality
- If a known issue was fixed, remove it from the doc and from docs/README.md
  aggregate list
- If a new known issue was discovered, add it
- Adjust ratings if the change improved or degraded the system

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
