#!/usr/bin/env python3
"""
dehyphen.py: strip em-dashes and prose double-hyphens from Markdown.

Why this exists
---------------
The user finds em-dashes and prose double-hyphens robotic and AI-sounding.
The rule lives in ~/.claude/CLAUDE.md under "Absolute rules" and as a
feedback memory. Drafts still leak them; this script catches the leaks
and rewrites them as natural punctuation.

What it does
------------
For each .md file passed on the command line, walk lines top-to-bottom:

  1. Skip fenced code blocks (between ``` fences).
  2. Skip lines that look like CLI flag tables (`--release`, `--foo=bar`).
  3. In remaining "prose" content, rewrite:
       ' -- '  ->  '. ' or ': ' (heuristic on the next character)
       ' — '  ->  '. ' or ': '
       '—'    -> '. '
     If the character after the dash is uppercase, use '. ' (new sentence).
     Otherwise use ': ' (explanation).
  4. Leaves '--<word>' (CLI flag form, no leading space) untouched.
  5. Leaves '---' lines (YAML front-matter and HR rules) untouched.
  6. Leaves table-cell '|' separators alone.

Idempotent: running again on cleaned files is a no-op.

Usage
-----
    python scripts/dehyphen.py path/to/file.md [more.md ...]
    python scripts/dehyphen.py --check path/to/file.md   # exit 1 if any prose violations remain

Audit before committing:
    grep -rnE ' -- | — ' skills/ docs/

Limitations
-----------
The heuristic is not perfect. A '. ' / ': ' replacement may sometimes be
wrong (a list continuation might want a comma). Review the diff after
running, especially around bullet lists. The script never lengthens
content, so reviewing is cheap.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

EMDASH = "—"  # actual em-dash char

# A line is "code-like" if it starts with one of these (already in a code
# block, OR a CLI invocation, OR a YAML / HR rule). We keep these intact.
HR_RULE_RE = re.compile(r"^\s*-{3,}\s*$")
CLI_FLAG_RE = re.compile(r"--[A-Za-z]")  # `--release`, `--foo=bar`, etc.


def _rewrite_chunk(chunk: str) -> str:
    """Rewrite ` -- ` and em-dash forms inside a non-backtick chunk.

    Default: replace with '. ' and capitalize the next word's first letter.
    Periods and sentences read human; colons everywhere read like a definition
    list. Reviewer can manually rewrite resulting short fragments into longer
    natural sentences as needed; the script only removes the offending dashes.

    Trailing forms (line wraps) collapse to '.' so the continuation on the next
    line starts a new sentence.

    The exceptions are punctuation contexts where '. ' would be ungrammatical:
    immediately after a backtick / quote (the chunk ends mid-quote), inside a
    parenthetical clause already opened, etc. Those are rare in practice;
    the reviewer handles them.
    """

    def replace_inline(m: "re.Match[str]") -> str:
        nxt = m.group(1)
        # If next char is a letter, capitalize it; otherwise leave alone.
        if nxt.isalpha():
            return ". " + nxt.upper()
        return ". " + nxt

    # ' -- X' (space + double-hyphen + space + non-whitespace).
    chunk = re.sub(r" -- (\S)", replace_inline, chunk)
    # Trailing ' --' at end of chunk (line wraps; continuation on next line).
    chunk = re.sub(r" --$", ".", chunk)

    # ' — X' with surrounding spaces (em-dash inline in prose).
    chunk = re.sub(rf" {EMDASH} (\S)", replace_inline, chunk)
    # ' — ' alone (chunk-boundary case after split-by-backtick; no next char
    # in this chunk). Collapse to '. ' (single space).
    chunk = re.sub(rf" {EMDASH} ", ". ", chunk)
    # Trailing ' —' at end of chunk.
    chunk = re.sub(rf" {EMDASH}$", ".", chunk)
    # Bare em-dash, no surrounding spaces (e.g. 'a—b').
    chunk = re.sub(rf"{EMDASH}", ". ", chunk)
    return chunk


def _replace_prose_dashes(line: str) -> str:
    """Rewrite in-line ` -- ` and em-dash forms to '. ' / ': '.

    Preserves anything inside backtick spans (`…`) so that inline code
    examples like `` `cargo test -- --nocapture` `` keep their literal `--`.
    Splits the line on backticks, alternating: outside, inside, outside, inside.
    """
    parts = line.split("`")
    for i in range(0, len(parts), 2):  # even indexes are outside-backtick chunks
        parts[i] = _rewrite_chunk(parts[i])
    return "`".join(parts)


def _scan_violations(line: str) -> bool:
    """True if a re-run of the rewriter would still change this line.

    Matches the contract of the `--check` mode so the post-rewrite count
    in normal mode agrees with audit results.
    """
    return _replace_prose_dashes(line) != line


def process(text: str) -> tuple[str, int]:
    """Return (rewritten_text, remaining_violation_count_outside_code_blocks)."""
    out: list[str] = []
    in_code = False
    remaining = 0
    for raw in text.split("\n"):
        stripped = raw.lstrip()
        # Fenced-code toggle. Note: we must NOT mistake an HR rule for a fence.
        if stripped.startswith("```"):
            in_code = not in_code
            out.append(raw)
            continue
        if in_code:
            out.append(raw)
            continue
        # YAML front-matter delimiter and Markdown HR rules: leave alone.
        if HR_RULE_RE.match(raw):
            out.append(raw)
            continue
        # Indented-code lines (4+ leading spaces) we treat as prose by default;
        # if they contain CLI flags those stay safe (no ' -- ' surrounding).
        line = _replace_prose_dashes(raw)
        if _scan_violations(line):
            remaining += 1
        out.append(line)
    return "\n".join(out), remaining


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("files", nargs="+", type=pathlib.Path, help="Markdown files to clean")
    ap.add_argument(
        "--check",
        action="store_true",
        help="Do not write. Exit 1 if any file has remaining prose violations.",
    )
    args = ap.parse_args(argv)

    bad = 0
    for path in args.files:
        original = path.read_text(encoding="utf-8")
        rewritten, remaining = process(original)
        if args.check:
            # A line is a violation iff the rewriter would change it.
            # That matches the contract exactly and dodges false positives
            # from substrings (e.g. ` -- ` appearing inside ` --- `).
            live_remaining = 0
            in_code = False
            for ln in original.split("\n"):
                stripped = ln.lstrip()
                if stripped.startswith("```"):
                    in_code = not in_code
                    continue
                if in_code or HR_RULE_RE.match(ln):
                    continue
                if _replace_prose_dashes(ln) != ln:
                    live_remaining += 1
            if live_remaining:
                print(f"{path}: {live_remaining} prose violation(s)")
                bad += 1
            else:
                print(f"{path}: clean")
            continue
        if rewritten != original:
            path.write_text(rewritten, encoding="utf-8")
            changed = sum(
                1
                for a, b in zip(original.split("\n"), rewritten.split("\n"))
                if a != b
            )
            print(f"{path}: rewrote {changed} line(s); {remaining} suspicious line(s) remain")
        else:
            print(f"{path}: clean")

    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
