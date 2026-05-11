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


def _replace_prose_dashes(line: str) -> str:
    """Rewrite in-line ` -- ` and em-dash forms to '. ' / ': '."""

    def pick(next_char: str) -> str:
        # New sentence if the next character is uppercase; explanation otherwise.
        if next_char and next_char.isupper():
            return ". "
        return ": "

    # ' -- X' and ' — X' (space + dash + space + non-whitespace).
    line = re.sub(r" -- (\S)", lambda m: pick(m.group(1)) + m.group(1), line)
    line = re.sub(rf" {EMDASH} (\S)", lambda m: pick(m.group(1)) + m.group(1), line)
    # Trailing ' --' at end of line (continuation on next line).
    # Replace with ':' so the wrap reads as a list continuation.
    line = re.sub(r" --$", ":", line)
    line = re.sub(rf" {EMDASH}$", ":", line)
    # Embedded em-dash with no spaces ('a—b'): treat as ': '.
    line = re.sub(rf"{EMDASH}", ": ", line)
    return line


def _scan_violations(line: str) -> bool:
    """True if the line still contains a prose ' -- ' or em-dash after rewrite."""
    if " -- " in line or EMDASH in line:
        return True
    return False


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
            # Count only prose violations (skip fenced code blocks + HR rules).
            live_remaining = 0
            in_code = False
            for ln in original.split("\n"):
                stripped = ln.lstrip()
                if stripped.startswith("```"):
                    in_code = not in_code
                    continue
                if in_code or HR_RULE_RE.match(ln):
                    continue
                if " -- " in ln or " --" == ln[-3:] or EMDASH in ln:
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
