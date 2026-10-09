# Which way to store a personal notebook: twelve stores, measured

**Status: measured on 2026-10-09. The choice is still open.**

onelog is a local notebook and project tracker: one YAML file per page
(daily logs, project pages, notes), read and written by a Rust desktop app.
The question was which way of storing its pages works best, decided from
data, not opinion: YAML files as today, JSON files, or a database. Twelve
stores were tried, all local on one PC, all from Rust, on the real notebook
(327 pages) and on ten copies of it (3270 pages). The program's raw output
is in [database-bench-output.txt](database-bench-output.txt); every number
below is copied from it.

## Findings

The app has to ask its questions (list a section by date, filter by
status, search text), so only stores that can answer them themselves are
ranked here: SQLite, Turso, DuckDB, PostgreSQL, SurrealDB, PoloDB and YAML +
tantivy. The stores without queries (YAML and JSON files, redb, fjall, LMDB)
stay in the tables below as reference points; YAML, what the app uses today,
is shown beside the winners. Times in ms, at the notebook's real size (327
pages) / at ten times it.

| Question | Best | Runner-up | Today (YAML) |
|---|---|---|---|
| Import every page | PoloDB 41 / 338 | SQLite 64 / 828 | 127 / 1328 |
| Open the daily section | SQLite 0.146 / 1.17 | Turso 0.222 at 1x, DuckDB 1.59 at 10x | 40.74 / 404 |
| Open one page | Turso 0.167 / 0.167 | SQLite 0.178 / 0.173 | 0.193 / 0.199 |
| Save one page | PostgreSQL 1.50 / 1.32 | SQLite 2.42 / 2.39 | 0.362 / 0.369 (not forced to disk) |
| Search every page for text | YAML + tantivy 0.52 at 1x, DuckDB 4.57 at 10x | Turso 0.63 at 1x, YAML + tantivy 6.49 at 10x | 144 / 1428 |
| Full-text search | YAML + tantivy 0.010 at 1x, SQLite 0.046 at 10x | SQLite 0.022 at 1x, YAML + tantivy 0.080 at 10x | none |
| Active projects | SQLite 0.014 / 0.112 | YAML + tantivy 0.029 / 0.257 | 22.01 / 223 |
| No server | all but PostgreSQL | | |
| **Overall for this app** | **SQLite** | PostgreSQL | |

**Why SQLite overall:** it wins three questions outright (the daily
section, active projects, full-text search at 10x) and is second on three
more (import, open, save), the only store in the top two on six of the
seven; it needs no server, keeps everything in one file, and forces every
save to disk. YAML + tantivy wins the searches but every save costs 50 ms.
PostgreSQL wins the save (1.5 against 2.4 ms) and needs its server running;
it would be the pick if many users or machines shared one notebook. PoloDB
imports fastest but never forces a save to disk and has an index bug.

## How each store was set up

Every database uses one schema: `page` holds what lists and filters read
(section, title, status, created, updated), `body` holds each page's text
and words. The candidates beyond YAML, SQLite, PostgreSQL and SurrealDB came
from a GitHub search for maintained, embeddable stores.

| Store | How it runs here | Full-text index |
|---|---|---|
| YAML | one file per page, read and written by onelog's own code, as onelog runs today | none |
| JSON | one file per page holding the same fields the databases store | none |
| YAML + tantivy | the YAML files, with a tantivy index beside them answering lists and searches | tantivy |
| SQLite | one file through `rusqlite` (SQLite built in), WAL, `synchronous = FULL` | FTS5, kept by triggers |
| Turso 0.8.2 | SQLite rewritten in Rust, one file | Turso's own (`USING fts`) |
| DuckDB | in-process SQL built for analysis, one file | none (an extension the crate cannot build in) |
| PostgreSQL 18 | a server on the same PC, default settings | stored `tsvector` column, GIN |
| SurrealDB 3 | built in, SurrealKV disk store, `sync = every` | FULLTEXT, BM25 |
| PoloDB | embedded MongoDB-style document database on RocksDB, one file | none |
| redb | pure Rust key-value store (B-trees), one file | none |
| fjall | pure Rust key-value store (log-structured, like RocksDB) | none |
| LMDB (heed) | memory-mapped key-value store | none |

The key-value stores (redb, fjall, LMDB) have no queries; they share one
piece of code that keeps its own index entries, so their numbers include how
that code is written.

## What is timed

What the app asks of its storage:

- import every page
- open the daily section, newest first
- open one page and read it into the app's page type
- save one page
- search every page for a piece of text, any case
- search through each store's full-text index
- list the active projects

Before any timing counts, a test loads the notebook twice over into every
store and checks each gives the YAML files' answers: all twelve give 476
daily pages, 10 pages containing "onelog" and 36 active projects, and every
full-text index finds the same 10 pages.

Each question is asked again and again for 0.3 s (at least 3 times, at most
50) and the middle time is reported; import runs once. The whole run takes
95 s.

## This PC

| | |
|---|---|
| CPU | Intel Core i7-9700K, 3.60 GHz |
| Memory | 32 GB |
| Disk | Samsung SSD 970 EVO Plus 2TB (NVMe) |
| System | Windows 10 Home N |
| Rust | rustc 1.98.0, release build without link-time optimization |

## Versions

| Store | Version |
|---|---|
| YAML | the app's own reader and writer (saphyr 0.1.0) |
| JSON | serde_json 1.0.151 |
| tantivy | 0.26.2 |
| SQLite | 3.53.2 (rusqlite 0.40.2) |
| Turso | turso 0.8.2 |
| DuckDB | duckdb crate 1.10506.0 |
| PostgreSQL | 18.6 (postgres crate 0.19.14) |
| SurrealDB | 3.3.2 |
| PoloDB | polodb_core 5.3.0 |
| redb | 4.3.0 |
| fjall | 3.1.12 |
| LMDB | heed 0.22.1 (lmdb-master-sys 0.2.6) |

## Times

The middle time of each question in milliseconds. A dash: the store has no
full-text index.

### The notebook as it is (327 pages)

| Store | Import | Daily section | Open a page | Save a page | Search every page | Full-text search | Active projects |
|---|---:|---:|---:|---:|---:|---:|---:|
| YAML (today) | 127 | 40.74 | 0.193 | 0.362 | 144 | - | 22.01 |
| JSON | 135 | 15.58 | 0.209 | 0.509 | 22.55 | - | 4.16 |
| YAML + tantivy | 260 | 0.362 | 0.191 | 50.09 | 0.519 | 0.010 | 0.029 |
| SQLite | 63.99 | 0.146 | 0.178 | 2.42 | 1.07 | 0.022 | 0.014 |
| Turso | 1251 | 0.222 | 0.167 | 6.13 | 0.629 | 0.064 | 0.056 |
| DuckDB | 1598 | 0.446 | 0.431 | 5.98 | 0.809 | - | 0.440 |
| PostgreSQL | 211 | 0.742 | 0.333 | 1.50 | 1.03 | 0.804 | 0.142 |
| SurrealDB | 325 | 3.81 | 0.280 | 2.70 | 2.74 | 0.303 | 0.523 |
| PoloDB | 40.54 | 5.01 | 0.217 | 5.73 | 4.51 | - | 2.80 |
| redb | 15.98 | 0.064 | 0.157 | 3.08 | 0.633 | - | 0.005 |
| fjall | 178 | 0.120 | 0.157 | 2.24 | 0.600 | - | 0.012 |
| LMDB | 54.94 | 0.055 | 0.158 | 0.465 | 0.610 | - | 0.003 |

### Ten times the notebook (3270 pages)

| Store | Import | Daily section | Open a page | Save a page | Search every page | Full-text search | Active projects |
|---|---:|---:|---:|---:|---:|---:|---:|
| YAML (today) | 1328 | 404 | 0.199 | 0.369 | 1428 | - | 223 |
| JSON | 1425 | 151 | 0.212 | 0.502 | 216 | - | 41.78 |
| YAML + tantivy | 1512 | 4.31 | 0.204 | 50.32 | 6.49 | 0.080 | 0.257 |
| SQLite | 828 | 1.17 | 0.173 | 2.39 | 19.70 | 0.046 | 0.112 |
| Turso | 16096 | 2.50 | 0.167 | 6.12 | 52.44 | 0.155 | 0.800 |
| DuckDB | 41807 | 1.59 | 0.459 | 9.62 | 4.57 | - | 0.684 |
| PostgreSQL | 2152 | 5.04 | 0.292 | 1.32 | 10.06 | 7.38 | 0.326 |
| SurrealDB | 2025 | 31.82 | 0.268 | 2.73 | 22.39 | 0.760 | 3.65 |
| PoloDB | 338 | 35.90 | 0.206 | 12.35 | 31.93 | - | 11.14 |
| redb | 100 | 0.596 | 0.160 | 3.64 | 8.85 | - | 0.037 |
| fjall | 250 | 1.39 | 0.161 | 2.20 | 9.26 | - | 0.066 |
| LMDB | 471 | 0.498 | 0.158 | 0.482 | 9.01 | - | 0.027 |

## What timing cannot show

| Store | Readable git diff | Fix a page by hand | Backup | Needs a server | Each save forced to disk | Queries | Full-text index |
|---|---|---|---|---|---|---|---|
| YAML (today) | yes | yes, any editor | copy the folder | no | no (temp file and rename) | none, the app's code | none |
| JSON | poor, a page is one line | yes, any editor | copy the folder | no | no (temp file and rename) | none | none |
| YAML + tantivy | yes, the files | yes, but the index must be rebuilt | the folder; the index can be rebuilt | no | files no, index commits | tantivy queries | tantivy |
| SQLite | no | with the sqlite3 tool | one file, SQLite's backup command | no | yes, `synchronous = FULL` | SQL | FTS5 |
| Turso | no | with Turso's tool | one file | no | yes | SQL | its own, still experimental in 0.8.2 |
| DuckDB | no | with the duckdb tool | one file | no | yes | SQL | an extension the crate cannot build in |
| PostgreSQL | no | with psql | pg_dump | yes | yes | SQL | built in |
| SurrealDB | no | with its own tools | its export | no (built in here) | yes, `sync = every` | SurrealQL | built in |
| PoloDB | no | no | one file | no | **no**, its code never syncs | MongoDB style | none |
| redb | no | no | one file | no | yes (its default) | none, own index keys | none |
| fjall | no | no | a folder | no | yes, `PersistMode::SyncAll` | none, own index keys | none |
| LMDB | no | no | one file | no | said to (its default); its 0.47 ms save was not checked against the disk | none, own index keys | none |

## What the numbers say

- **YAML, what the app uses today, is the slowest at everything that looks
  across pages, and grows with the notebook:** listing the daily section
  takes 41 ms today and 404 ms at ten times the pages; a search 144 ms and
  1.4 s. Opening or saving one page is as fast as anything here.
- **JSON files** are 2.6 to 6.4 times faster than YAML at the same work
  (parsing JSON is cheaper, and the listed fields are stored ready), but grow
  the same way.
- **SQLite is the only store with SQL and a full-text index that answers
  the daily section, a page, a save, a full-text search and the active
  projects in under 2.5 ms at both sizes**, in one file with no server, and
  forces every save to disk; its save (2.4 ms) is the cost of that. Its
  search of every page for any piece of text takes 1.1 ms today and 19.7 ms
  at ten times the pages, which is what the full-text index (0.05 ms) is for.
- **LMDB and redb** answer lists and lookups fastest, but have no queries
  and no full-text search: the app would write and keep every index itself.
- **YAML + tantivy** keeps the files and makes lists and search fast, but
  every save costs 50 ms (tantivy commits its index).
- **PostgreSQL** answers everything under 1.5 ms today but is behind SQLite
  on 5 of the 7 questions (the daily section 0.74 against 0.15 ms, 5.0
  against 1.2 ms at 10x); it wins the save (1.5 against 2.4 ms) and the
  search of every page (10.1 against 19.7 ms at 10x), needs its server
  running, and its full-text search ran 7.4 ms at 10x (0.8 ms at 1x), not
  explained.
- **Not a fit here:** DuckDB (built for analysis: 1.6 s and 42 s to import,
  6 to 10 ms a save), Turso (import 1.3 s and 16 s, its full-text index
  experimental), SurrealDB (31.8 ms to list the daily section at 10x, though
  its query plan uses its index), PoloDB (no forced saves, and a bug: with
  an index on a field, a sorted query or one on two fields returns nothing).

## Found on the way

- SQLite's FTS5 table first deleted index rows by an unindexed key, which
  read the whole index on every write: the import of 32700 pages took 17
  minutes. An external content table kept by triggers, as SQLite's FTS5
  documentation sets out, took it to 9 s.
- SurrealDB reads a whole record to give any of its fields, so with the page
  text in the same record as the listed fields, every listing read every
  page's text. Splitting `page` from `body` (now the schema every database
  shares) helped every store.
- PostgreSQL's full-text search recomputed every row's words until the
  words got a stored `tsvector` column, as its documentation recommends.

Not measured: YAML files kept as the pages with a SQLite index beside them,
the way YAML + tantivy is measured here.
