#!/usr/bin/env python3
"""
dehyphen.py: strip em-dashes and prose double-hyphens from text files.

Why this exists
---------------
The user finds em-dashes and prose double-hyphens robotic and AI-sounding.
The rule lives in ~/.claude/CLAUDE.md under "Absolute rules" and as a
feedback memory. Drafts still leak them; this script catches the leaks
and rewrites them as natural punctuation.

Languages supported
-------------------
- `markdown` (default for .md files). Skips fenced code blocks, HR rules,
  YAML front-matter, inline backtick code spans, and ` -- ` inside
  CLI flag patterns. Rewrites prose only.
- `rust` (default for .rs files). Tokenizes the file to find line comments,
  block comments, and doc-comments. Rewrites prose only inside comments.
  Never touches code, string literals, raw strings, or char literals.
- `plain` (default for .txt). Treats the whole file as prose.

Rewrite rule (any language)
---------------------------
Default replacement for both ` -- ` and ` — ` is '. ' with the next word's
first letter capitalized. Trailing ` --` / ` —` at line end becomes '.'.
Bare em-dash with no surrounding spaces becomes '. ' as well.
CLI flag forms like `--release` (no leading space) stay untouched.

Idempotent. Running again on cleaned files is a no-op.

Usage
-----
    python scripts/dehyphen.py path/to/file
    python scripts/dehyphen.py --check path/to/file        # exit 1 on violations
    python scripts/dehyphen.py --lang rust path/to/file.rs # override auto-detect

Audit before committing:
    python scripts/dehyphen.py --check $(git diff --cached --name-only)

Limitations
-----------
- Rust: char-literal detection is best-effort. Lifetimes vs char literals
  is the usual ambiguity. We err on the side of NOT treating ambiguous
  `'X` constructs as char literals (they're then walked as code, which is
  the safe choice because code is never rewritten anyway).
- Rust: a `/` inside an unclosed line comment at the very end of file
  with no trailing newline gets handled correctly.
- The heuristic-driven rewrite may sometimes pick a less-natural
  punctuation. Review the diff after running. The script never lengthens
  content, so reviewing is cheap.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

EMDASH = "—"

# A line is treated as an HR rule (and skipped in markdown mode) if it
# matches /^\s*---+\s*$/. This also catches YAML front-matter delimiters.
HR_RULE_RE = re.compile(r"^\s*-{3,}\s*$")

# Char literal recognizer for Rust: an opening ' followed by either a
# single non-quote, non-backslash char, or a backslash escape, then closing '.
# Conservative: only matches obvious char literals. Anything that doesn't
# match here gets treated as a lifetime / regular code.
RUST_CHAR_LITERAL_RE = re.compile(
    r"""'(?:\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|\\[\\nrt0'"]|[^'\\\n])'"""
)

# Raw string opener: r followed by zero or more '#' then '"'.
RUST_RAW_STRING_OPEN_RE = re.compile(r"r(#*)\"")


def _rewrite_line(line: str) -> str:
    """Rewrite ` -- ` and em-dash forms on a single line of prose.

    Default: replace with '. ' and capitalize the next word's first letter.
    Trailing forms collapse to '.' so the continuation on the next line
    starts a new sentence. Bare em-dash with no surrounding spaces gets '. '.
    """

    def replace_inline(m: "re.Match[str]") -> str:
        nxt = m.group(1)
        if nxt.isalpha():
            return ". " + nxt.upper()
        return ". " + nxt

    # `<space><dash><dash><space><nonspace>` form.
    line = re.sub(r" -- (\S)", replace_inline, line)
    # Trailing `<space><dash><dash>` at end of line (continuation wraps).
    line = re.sub(r" --$", ".", line)
    # `<space><emdash><space><nonspace>` inline form.
    line = re.sub(rf" {EMDASH} (\S)", replace_inline, line)
    # Bare `<space><emdash><space>` with nothing useful after (chunk boundary).
    line = re.sub(rf" {EMDASH} ", ". ", line)
    # Trailing em-dash at end of line.
    line = re.sub(rf" {EMDASH}$", ".", line)
    # Bare em-dash with no surrounding spaces (e.g. `a<emdash>b`).
    line = re.sub(rf"{EMDASH}", ". ", line)
    return line


def _rewrite_prose(text: str) -> str:
    """Rewrite a stretch of prose, preserving inline backtick code spans.

    Splits on backticks; only rewrites outside-backtick chunks.
    """
    parts = text.split("`")
    for i in range(0, len(parts), 2):
        # Process this chunk line-by-line so trailing `$` regexes match
        # end-of-line, not just end-of-chunk.
        lines = parts[i].split("\n")
        parts[i] = "\n".join(_rewrite_line(ln) for ln in lines)
    return "`".join(parts)


def _line_would_change(line: str) -> bool:
    """True if a re-run of the rewriter would change this line."""
    return _rewrite_prose(line) != line


# ----------------------------------------------------------------------
# Markdown processor
# ----------------------------------------------------------------------


def _process_markdown(text: str) -> tuple[str, int]:
    """Return (rewritten_text, violation_count_outside_code_blocks)."""
    out: list[str] = []
    in_code = False
    remaining = 0
    for raw in text.split("\n"):
        stripped = raw.lstrip()
        if stripped.startswith("```"):
            in_code = not in_code
            out.append(raw)
            continue
        if in_code or HR_RULE_RE.match(raw):
            out.append(raw)
            continue
        line = _rewrite_prose(raw)
        if _line_would_change(line):
            remaining += 1
        out.append(line)
    return "\n".join(out), remaining


def _check_markdown(text: str) -> int:
    """Count lines that would change. Mirrors the rewrite contract."""
    count = 0
    in_code = False
    for ln in text.split("\n"):
        stripped = ln.lstrip()
        if stripped.startswith("```"):
            in_code = not in_code
            continue
        if in_code or HR_RULE_RE.match(ln):
            continue
        if _line_would_change(ln):
            count += 1
    return count


# ----------------------------------------------------------------------
# Rust processor
# ----------------------------------------------------------------------


def _tokenize_rust(text: str) -> list[tuple[str, str]]:
    """Split a Rust source file into (kind, text) tokens.

    Kinds: 'code', 'line_comment', 'block_comment', 'string', 'raw_string',
    'char'. Only the comment kinds get rewritten by the caller.

    Best-effort tokenizer. It tracks just enough state to keep prose
    rewriting from leaking into string literals or code. Handles nested
    block comments per the Rust reference.
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))
        code_start = -1  # invalidate; caller will reset when re-entering code

    while i < n:
        c = text[i]

        # Block comment (nested per Rust reference).
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            flush_code(i)
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if text[j] == "/" and j + 1 < n and text[j + 1] == "*":
                    depth += 1
                    j += 2
                elif text[j] == "*" and j + 1 < n and text[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append(("block_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Line comment (incl. doc forms /// and //!).
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            flush_code(i)
            j = text.find("\n", i)
            if j < 0:
                j = n
            out.append(("line_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Raw string r#*"..."#*. Must not be preceded by an identifier char.
        if c == "r" and i + 1 < n and text[i + 1] in '#"':
            prev_ok = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
            if prev_ok:
                m = RUST_RAW_STRING_OPEN_RE.match(text, i)
                if m:
                    flush_code(i)
                    hashes = m.group(1)
                    close = '"' + hashes
                    j = text.find(close, m.end())
                    if j < 0:
                        j = n
                    else:
                        j += len(close)
                    out.append(("raw_string", text[i:j]))
                    i = j
                    code_start = i
                    continue

        # Regular string literal.
        if c == '"':
            flush_code(i)
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                elif text[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        # Char literal (or lifetime). Only consume if it matches our
        # conservative char-literal regex; otherwise treat as code.
        if c == "'":
            m = RUST_CHAR_LITERAL_RE.match(text, i)
            if m:
                flush_code(i)
                out.append(("char", m.group(0)))
                i = m.end()
                code_start = i
                continue

        i += 1

    # Flush any trailing code.
    if code_start >= 0 and code_start < n:
        out.append(("code", text[code_start:]))
    return out


def _process_rust(text: str) -> tuple[str, int]:
    tokens = _tokenize_rust(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind in ("line_comment", "block_comment"):
            # Process line-by-line so '$' trailing patterns work as expected
            # across multi-line block comments.
            lines = chunk.split("\n")
            new_lines = [_rewrite_prose(ln) for ln in lines]
            new_chunk = "\n".join(new_lines)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_rust(text: str) -> int:
    tokens = _tokenize_rust(text)
    count = 0
    for kind, chunk in tokens:
        if kind not in ("line_comment", "block_comment"):
            continue
        for ln in chunk.split("\n"):
            if _line_would_change(ln):
                count += 1
    return count


# ----------------------------------------------------------------------
# Python processor
# ----------------------------------------------------------------------


def _tokenize_python(text: str) -> list[tuple[str, str]]:
    """Split a Python source file into (kind, text) tokens.

    Kinds: 'code', 'line_comment', 'string'. Only line_comment gets
    rewritten by the caller. Triple-strings and single-line strings
    (including prefixed r/b/f/u variants) are all kept intact under
    'string'; conservatively never rewrite inside any string literal.

    Best-effort. Handles: # line comments, '...' / "..." single-line
    strings with backslash escapes, '''...''' / \"\"\"...\"\"\" triple
    strings, prefixed strings (r/R/b/B/f/F/u/U + 1-2 char combinations).
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))

    def _string_prefix_len(pos: int) -> int:
        # Look at up to 2 chars before `pos` to detect a string prefix
        # like r, b, f, rb, br, fr, rf, etc. The prefix must be a complete
        # identifier (i.e. NOT preceded by another identifier char).
        prefix_chars = "rRbBfFuU"
        plen = 0
        if pos - 1 >= 0 and text[pos - 1] in prefix_chars:
            plen = 1
            if pos - 2 >= 0 and text[pos - 2] in prefix_chars:
                plen = 2
        if plen > 0:
            preceding = pos - plen - 1
            if preceding >= 0 and (text[preceding].isalnum() or text[preceding] == "_"):
                plen = 0
        return plen

    while i < n:
        c = text[i]

        # `#` line comment.
        if c == "#":
            flush_code(i)
            j = text.find("\n", i)
            if j < 0:
                j = n
            out.append(("line_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Triple-quoted string.
        if text[i : i + 3] in ('"""', "'''"):
            plen = _string_prefix_len(i)
            flush_code(i - plen)
            quote = text[i : i + 3]
            j = text.find(quote, i + 3)
            if j < 0:
                j = n
            else:
                j += 3
            out.append(("string", text[i - plen : j]))
            i = j
            code_start = i
            continue

        # Single-line string. Handles escape sequences and stops at unescaped
        # newline (Python doesn't allow unescaped newlines in single-line strings).
        if c in ('"', "'"):
            plen = _string_prefix_len(i)
            flush_code(i - plen)
            quote = c
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                elif text[j] == "\n":
                    break  # unterminated, bail
                elif text[j] == quote:
                    j += 1
                    break
                else:
                    j += 1
            out.append(("string", text[i - plen : j]))
            i = j
            code_start = i
            continue

        i += 1

    flush_code(n)
    return out


def _process_python(text: str) -> tuple[str, int]:
    tokens = _tokenize_python(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind == "line_comment":
            new_chunk = _rewrite_prose(chunk)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_python(text: str) -> int:
    tokens = _tokenize_python(text)
    return sum(
        1
        for kind, chunk in tokens
        if kind == "line_comment" and _line_would_change(chunk)
    )


# ----------------------------------------------------------------------
# C# processor
# ----------------------------------------------------------------------


def _tokenize_csharp(text: str) -> list[tuple[str, str]]:
    """Split a C# source file into (kind, text) tokens.

    Kinds: 'code', 'line_comment', 'block_comment', 'string'. Only the
    comment kinds get rewritten. C# block comments do NOT nest (unlike
    Rust). String forms handled:

      - "..."           regular (backslash escapes)
      - @"..."          verbatim ("" for embedded ", no backslash escapes)
      - $"..."          interpolated
      - $@"..." / @$"..."  interpolated verbatim
      - "..."           C# 11+ raw strings (3+ quotes) handled by triple form
      - 'x'             char literal (handled as a 3-char span, conservative)
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))

    def _csharp_prefix_len(pos: int) -> int:
        # Look at up to 2 chars before pos for a string prefix: @, $, @$, $@.
        prefix_chars = "@$"
        plen = 0
        if pos - 1 >= 0 and text[pos - 1] in prefix_chars:
            plen = 1
            if pos - 2 >= 0 and text[pos - 2] in prefix_chars and text[pos - 2] != text[pos - 1]:
                plen = 2
        if plen > 0:
            preceding = pos - plen - 1
            if preceding >= 0 and (text[preceding].isalnum() or text[preceding] == "_"):
                plen = 0
        return plen

    while i < n:
        c = text[i]

        # Block comment (not nested in C#).
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            flush_code(i)
            j = text.find("*/", i + 2)
            if j < 0:
                j = n
            else:
                j += 2
            out.append(("block_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Line comment (incl. /// doc-comment).
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            flush_code(i)
            j = text.find("\n", i)
            if j < 0:
                j = n
            out.append(("line_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # C# 11+ raw string literal (3 or more quotes). Conservative: only
        # exactly three quotes handled; 4+ quote raw strings are rare.
        if text[i : i + 3] == '"""':
            plen = _csharp_prefix_len(i)
            flush_code(i - plen)
            j = text.find('"""', i + 3)
            if j < 0:
                j = n
            else:
                j += 3
            out.append(("string", text[i - plen : j]))
            i = j
            code_start = i
            continue

        # Verbatim string @"..." (also $@"..." / @$"..."). In a verbatim
        # string, `""` is an embedded quote; backslashes are literal.
        # We rely on the prefix detection to set the right span.
        if c == '"':
            plen = _csharp_prefix_len(i)
            verbatim = plen > 0 and "@" in text[i - plen : i]
            flush_code(i - plen)
            j = i + 1
            if verbatim:
                while j < n:
                    if text[j] == '"':
                        if j + 1 < n and text[j + 1] == '"':
                            j += 2  # escaped embedded quote
                        else:
                            j += 1
                            break
                    else:
                        j += 1
            else:
                # Regular interpolated or plain string. Backslash escapes.
                while j < n:
                    if text[j] == "\\" and j + 1 < n:
                        j += 2
                    elif text[j] == "\n":
                        break
                    elif text[j] == '"':
                        j += 1
                        break
                    else:
                        j += 1
            out.append(("string", text[i - plen : j]))
            i = j
            code_start = i
            continue

        # Char literal. Conservative match: same as Rust's, plus the C#
        # surrogate escapes (e.g. '\uXXXX'). Anything ambiguous falls
        # through to code.
        if c == "'":
            m = re.match(
                r"""'(?:\\u[0-9a-fA-F]{4}|\\x[0-9a-fA-F]{1,4}|\\[\\nrt0'"abfv]|[^'\\\n])'""",
                text[i:],
            )
            if m:
                flush_code(i)
                out.append(("char", m.group(0)))
                i += len(m.group(0))
                code_start = i
                continue

        i += 1

    flush_code(n)
    return out


def _process_csharp(text: str) -> tuple[str, int]:
    tokens = _tokenize_csharp(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind in ("line_comment", "block_comment"):
            lines = chunk.split("\n")
            new_chunk = "\n".join(_rewrite_prose(ln) for ln in lines)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_csharp(text: str) -> int:
    tokens = _tokenize_csharp(text)
    count = 0
    for kind, chunk in tokens:
        if kind not in ("line_comment", "block_comment"):
            continue
        for ln in chunk.split("\n"):
            if _line_would_change(ln):
                count += 1
    return count


# ----------------------------------------------------------------------
# Go processor
# ----------------------------------------------------------------------


def _tokenize_go(text: str) -> list[tuple[str, str]]:
    """Tokenize a Go source file.

    Kinds: 'code', 'line_comment', 'block_comment', 'string', 'raw_string',
    'char'. Only the comment kinds get rewritten.

    Go specifics:
      - // line comment (godoc above decl, regular elsewhere).
      - /* block comment */ (NOT nested).
      - "..." interpreted string (backslash escapes).
      - `...` raw string (backticks, may span lines, no escapes).
      - 'x' rune literal (a single rune or escape sequence).
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))

    while i < n:
        c = text[i]

        # Block comment.
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            flush_code(i)
            j = text.find("*/", i + 2)
            if j < 0:
                j = n
            else:
                j += 2
            out.append(("block_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Line comment.
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            flush_code(i)
            j = text.find("\n", i)
            if j < 0:
                j = n
            out.append(("line_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Raw string (backticks).
        if c == "`":
            flush_code(i)
            j = text.find("`", i + 1)
            if j < 0:
                j = n
            else:
                j += 1
            out.append(("raw_string", text[i:j]))
            i = j
            code_start = i
            continue

        # Interpreted string.
        if c == '"':
            flush_code(i)
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                elif text[j] == "\n":
                    break
                elif text[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        # Rune literal.
        if c == "'":
            m = re.match(
                r"""'(?:\\u[0-9a-fA-F]{4}|\\U[0-9a-fA-F]{8}|\\x[0-9a-fA-F]{2}|\\[0-7]{1,3}|\\[\\nrt0'"abfv]|[^'\\\n])'""",
                text[i:],
            )
            if m:
                flush_code(i)
                out.append(("char", m.group(0)))
                i += len(m.group(0))
                code_start = i
                continue

        i += 1

    flush_code(n)
    return out


def _process_go(text: str) -> tuple[str, int]:
    tokens = _tokenize_go(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind in ("line_comment", "block_comment"):
            lines = chunk.split("\n")
            new_chunk = "\n".join(_rewrite_prose(ln) for ln in lines)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_go(text: str) -> int:
    tokens = _tokenize_go(text)
    count = 0
    for kind, chunk in tokens:
        if kind not in ("line_comment", "block_comment"):
            continue
        for ln in chunk.split("\n"):
            if _line_would_change(ln):
                count += 1
    return count


# ----------------------------------------------------------------------
# TOML processor
# ----------------------------------------------------------------------


def _tokenize_toml(text: str) -> list[tuple[str, str]]:
    """Tokenize a TOML source file.

    Kinds: 'code', 'line_comment', 'string'. Only line_comment gets rewritten.

    TOML string forms handled (triple-double-quoted and triple-single-quoted
    multi-line strings, plus single-double-quoted basic and
    single-single-quoted literal strings).
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))

    while i < n:
        c = text[i]

        # # line comment.
        if c == "#":
            flush_code(i)
            j = text.find("\n", i)
            if j < 0:
                j = n
            out.append(("line_comment", text[i:j]))
            i = j
            code_start = i
            continue

        # Triple-quoted string.
        if text[i : i + 3] in ('"""', "'''"):
            flush_code(i)
            quote = text[i : i + 3]
            j = text.find(quote, i + 3)
            if j < 0:
                j = n
            else:
                j += 3
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        # Single-line string.
        if c in ('"', "'"):
            flush_code(i)
            quote = c
            j = i + 1
            if quote == '"':
                while j < n:
                    if text[j] == "\\" and j + 1 < n:
                        j += 2
                    elif text[j] in ('"', "\n"):
                        if text[j] == '"':
                            j += 1
                        break
                    else:
                        j += 1
            else:
                # Literal string: no escapes; ends at next ' or newline.
                while j < n and text[j] not in ("'", "\n"):
                    j += 1
                if j < n and text[j] == "'":
                    j += 1
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        i += 1

    flush_code(n)
    return out


def _process_toml(text: str) -> tuple[str, int]:
    tokens = _tokenize_toml(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind == "line_comment":
            new_chunk = _rewrite_prose(chunk)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_toml(text: str) -> int:
    tokens = _tokenize_toml(text)
    return sum(
        1 for kind, chunk in tokens if kind == "line_comment" and _line_would_change(chunk)
    )


# ----------------------------------------------------------------------
# Shell processor
# ----------------------------------------------------------------------


def _tokenize_shell(text: str) -> list[tuple[str, str]]:
    """Tokenize a POSIX-ish shell script.

    Kinds: 'code', 'line_comment', 'string'. Only line_comment is rewritten.

    Best-effort. Handles `#` comments, `"..."` double-quoted strings,
    and `'...'` single-quoted (literal) strings. Backtick command
    substitution and `$(...)` are treated as code. Heredoc bodies are
    treated as code, which means prose violations inside heredocs are
    missed but also never mangled. Pre-commit hook will catch those if
    they matter.
    """
    i = 0
    n = len(text)
    out: list[tuple[str, str]] = []
    code_start = 0

    def flush_code(end: int) -> None:
        nonlocal code_start
        if end > code_start:
            out.append(("code", text[code_start:end]))

    while i < n:
        c = text[i]

        # `#` line comment. Must be at start-of-line or after whitespace
        # (otherwise it's part of a parameter expansion like `${foo#bar}`).
        if c == "#":
            is_comment = i == 0 or text[i - 1] in (" ", "\t", "\n")
            if is_comment:
                flush_code(i)
                j = text.find("\n", i)
                if j < 0:
                    j = n
                out.append(("line_comment", text[i:j]))
                i = j
                code_start = i
                continue

        # Single-quoted literal string (no escapes; ends at next ').
        if c == "'":
            flush_code(i)
            j = text.find("'", i + 1)
            if j < 0:
                j = n
            else:
                j += 1
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        # Double-quoted string (backslash escapes, but `#` is literal).
        if c == '"':
            flush_code(i)
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                elif text[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            out.append(("string", text[i:j]))
            i = j
            code_start = i
            continue

        i += 1

    flush_code(n)
    return out


def _process_shell(text: str) -> tuple[str, int]:
    tokens = _tokenize_shell(text)
    out_parts: list[str] = []
    remaining = 0
    for kind, chunk in tokens:
        if kind == "line_comment":
            new_chunk = _rewrite_prose(chunk)
            if _line_would_change(new_chunk):
                remaining += 1
            out_parts.append(new_chunk)
        else:
            out_parts.append(chunk)
    return "".join(out_parts), remaining


def _check_shell(text: str) -> int:
    tokens = _tokenize_shell(text)
    return sum(
        1
        for kind, chunk in tokens
        if kind == "line_comment" and _line_would_change(chunk)
    )


# ----------------------------------------------------------------------
# Plain processor (whole file is prose)
# ----------------------------------------------------------------------


def _process_plain(text: str) -> tuple[str, int]:
    out_lines: list[str] = []
    remaining = 0
    for ln in text.split("\n"):
        new = _rewrite_prose(ln)
        if _line_would_change(new):
            remaining += 1
        out_lines.append(new)
    return "\n".join(out_lines), remaining


def _check_plain(text: str) -> int:
    return sum(1 for ln in text.split("\n") if _line_would_change(ln))


# ----------------------------------------------------------------------
# Dispatch
# ----------------------------------------------------------------------


LANG_BY_EXT = {
    ".md": "markdown",
    ".markdown": "markdown",
    ".rs": "rust",
    ".py": "python",
    ".pyi": "python",
    ".cs": "csharp",
    ".go": "go",
    ".toml": "toml",
    ".sh": "shell",
    ".bash": "shell",
    ".txt": "plain",
    ".rst": "plain",
}


def _detect_lang(path: pathlib.Path) -> str:
    return LANG_BY_EXT.get(path.suffix.lower(), "markdown")


PROCESSORS = {
    "markdown": _process_markdown,
    "rust": _process_rust,
    "python": _process_python,
    "csharp": _process_csharp,
    "go": _process_go,
    "toml": _process_toml,
    "shell": _process_shell,
    "plain": _process_plain,
}

CHECKERS = {
    "markdown": _check_markdown,
    "rust": _check_rust,
    "python": _check_python,
    "csharp": _check_csharp,
    "go": _check_go,
    "toml": _check_toml,
    "shell": _check_shell,
    "plain": _check_plain,
}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("files", nargs="+", type=pathlib.Path, help="Files to clean")
    ap.add_argument(
        "--check",
        action="store_true",
        help="Do not write. Exit 1 if any file has remaining prose violations.",
    )
    ap.add_argument(
        "--lang",
        choices=sorted(PROCESSORS.keys()),
        default=None,
        help="Override auto-detect (by extension). Default: markdown for unknown extensions.",
    )
    args = ap.parse_args(argv)

    bad = 0
    for path in args.files:
        lang = args.lang or _detect_lang(path)
        original = path.read_text(encoding="utf-8")
        if args.check:
            live_remaining = CHECKERS[lang](original)
            if live_remaining:
                print(f"{path}: {live_remaining} prose violation(s) [{lang}]")
                bad += 1
            else:
                print(f"{path}: clean [{lang}]")
            continue
        rewritten, remaining = PROCESSORS[lang](original)
        if rewritten != original:
            path.write_text(rewritten, encoding="utf-8")
            changed = sum(
                1
                for a, b in zip(original.split("\n"), rewritten.split("\n"))
                if a != b
            )
            print(
                f"{path}: rewrote {changed} line(s); "
                f"{remaining} suspicious line(s) remain [{lang}]"
            )
        else:
            print(f"{path}: clean [{lang}]")

    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
