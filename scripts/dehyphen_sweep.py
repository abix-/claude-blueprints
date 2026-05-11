#!/usr/bin/env python3
"""
dehyphen_sweep.py: run dehyphen across one or more git repos.

What it does
------------
For each repo:
  1. Enumerate tracked files via `git ls-files` (respects .gitignore,
     never touches untracked files, never touches vendored content
     that's been excluded from the index).
  2. Filter to file types dehyphen.py supports.
  3. Skip files that have unstaged modifications (won't risk mixing
     the user's in-progress work with the sweep).
  4. In --check mode (default): print per-repo violation summary.
  5. In --apply mode: rewrite each violating file and create one
     commit per file with a standard message. Push at the end.

The script is idempotent. Re-runs are no-ops after a clean sweep.

Usage
-----
    python dehyphen_sweep.py --all                  # check every known repo
    python dehyphen_sweep.py /path/to/repo          # check one repo
    python dehyphen_sweep.py --apply --all          # rewrite + commit + push
    python dehyphen_sweep.py --apply --no-push <r>  # commit but don't push

Safety
------
- Default is --check (no writes).
- --apply commits per-file with `git commit -- <path>` so the commit is
  scoped (doesn't sweep up unrelated staged work).
- Files with unstaged modifications are SKIPPED. Stage or stash them
  first if you want them included.
- Untracked files are SKIPPED (we can't commit them anyway; user
  decides when to add).
"""

from __future__ import annotations

import argparse
import pathlib
import subprocess
import sys

# Languages dehyphen.py knows. Keep in sync with dehyphen.py's LANG_BY_EXT.
SUPPORTED_EXTS = {
    ".md", ".markdown",
    ".rs",
    ".py", ".pyi",
    ".cs",
    ".go",
    ".toml",
    ".sh", ".bash",
    ".js", ".mjs", ".cjs", ".ts", ".tsx", ".jsx",
    ".wgsl",
    ".yaml", ".yml",
    ".txt", ".rst",
}

# Default set of owned repos. Override with positional args.
# Paths are Windows-form to match this account's layout; on other
# hosts pass repo paths explicitly.
DEFAULT_REPOS = [
    "C:/code/claude-blueprints",
    "C:/code/grounded2mods",
    "C:/code/abixio",
    "C:/code/abixio-ui",
    "C:/code/endless",
    "C:/code/k3sc",
    "C:/code/Schedule1Mods",
    "C:/code/chromium-extensions",
]


def _run(args: list[str], cwd: pathlib.Path) -> str:
    res = subprocess.run(args, cwd=str(cwd), capture_output=True, text=True, check=False)
    return res.stdout


def _git_tracked(repo: pathlib.Path) -> list[pathlib.Path]:
    out = _run(["git", "ls-files"], repo)
    return [repo / line for line in out.splitlines() if line]


def _git_unstaged(repo: pathlib.Path) -> set[str]:
    """Set of repo-relative paths with unstaged modifications."""
    out = _run(["git", "diff", "--name-only"], repo)
    return {line for line in out.splitlines() if line}


def _check_file(dehyphen_path: pathlib.Path, target: pathlib.Path) -> int:
    """Return remaining-violation count for `target`, or 0 if clean."""
    res = subprocess.run(
        [sys.executable, str(dehyphen_path), "--check", str(target)],
        capture_output=True,
        text=True,
        check=False,
    )
    line = res.stdout.strip().splitlines()[-1] if res.stdout.strip() else ""
    if "prose violation" in line:
        # format: "path: N prose violation(s) [lang]"
        try:
            return int(line.split(":")[1].strip().split()[0])
        except (IndexError, ValueError):
            return 0
    return 0


def _apply_file(dehyphen_path: pathlib.Path, target: pathlib.Path) -> int:
    """Rewrite `target` via dehyphen. Return change count."""
    subprocess.run(
        [sys.executable, str(dehyphen_path), str(target)],
        capture_output=True,
        text=True,
        check=False,
    )
    # Re-check to confirm clean.
    return _check_file(dehyphen_path, target)


def _commit_file(repo: pathlib.Path, rel_path: str, count: int, lang: str) -> bool:
    """Per-file commit. Returns True if a commit was made."""
    msg = f"dehyphen: sweep {rel_path} ({count} lines, {lang})"
    res = subprocess.run(
        ["git", "commit", "-q", "-m", msg, "--", rel_path],
        cwd=str(repo),
        capture_output=True,
        text=True,
        check=False,
    )
    return res.returncode == 0


def _push(repo: pathlib.Path) -> bool:
    res = subprocess.run(
        ["git", "push"], cwd=str(repo), capture_output=True, text=True, check=False
    )
    return res.returncode == 0


def _detect_lang(path: pathlib.Path) -> str:
    ext = path.suffix.lower()
    mapping = {
        ".md": "md", ".markdown": "md",
        ".rs": "rust",
        ".py": "python", ".pyi": "python",
        ".cs": "csharp",
        ".go": "go",
        ".toml": "toml",
        ".sh": "shell", ".bash": "shell",
        ".js": "js", ".mjs": "js", ".cjs": "js", ".ts": "ts",
        ".tsx": "tsx", ".jsx": "jsx",
        ".wgsl": "wgsl",
        ".yaml": "yaml", ".yml": "yaml",
        ".txt": "plain", ".rst": "plain",
    }
    return mapping.get(ext, "plain")


def sweep_repo(repo: pathlib.Path, dehyphen: pathlib.Path, apply: bool, push: bool) -> dict:
    if not (repo / ".git").exists():
        return {"repo": str(repo), "skipped": "not a git repo", "files": 0, "violations": 0}

    tracked = _git_tracked(repo)
    unstaged = _git_unstaged(repo)

    files: list[pathlib.Path] = []
    for f in tracked:
        if f.suffix.lower() not in SUPPORTED_EXTS:
            continue
        if not f.is_file():
            continue
        files.append(f)

    total_violations = 0
    swept = 0
    skipped_dirty = 0
    failed_commit = 0

    for f in files:
        rel = f.relative_to(repo).as_posix()
        if rel in unstaged:
            skipped_dirty += 1
            continue
        n = _check_file(dehyphen, f)
        if n == 0:
            continue
        total_violations += n
        if apply:
            remaining = _apply_file(dehyphen, f)
            if remaining > 0:
                print(f"  [WARN] {rel}: {remaining} violations remain after rewrite")
            lang = _detect_lang(f)
            if _commit_file(repo, rel, n, lang):
                print(f"  [OK] {rel} ({n} lines, {lang})")
                swept += 1
            else:
                failed_commit += 1

    pushed = False
    if apply and swept > 0 and push:
        pushed = _push(repo)

    return {
        "repo": str(repo),
        "files_violating": len([
            f for f in files
            if f.relative_to(repo).as_posix() not in unstaged
            and _check_file(dehyphen, f) > 0
        ]) if not apply else 0,
        "violations": total_violations,
        "swept": swept,
        "skipped_dirty": skipped_dirty,
        "failed_commit": failed_commit,
        "pushed": pushed,
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "repos",
        nargs="*",
        type=pathlib.Path,
        help="Repos to sweep. Default: --all when no positional args.",
    )
    ap.add_argument("--all", action="store_true", help="Sweep the default repo list.")
    ap.add_argument(
        "--apply",
        action="store_true",
        help="Rewrite + commit. Default is --check (dry run).",
    )
    ap.add_argument(
        "--no-push",
        action="store_true",
        help="With --apply, do NOT push after the commits. Default: push.",
    )
    ap.add_argument(
        "--dehyphen",
        type=pathlib.Path,
        default=pathlib.Path(__file__).parent / "dehyphen.py",
        help="Path to dehyphen.py (default: sibling).",
    )
    args = ap.parse_args(argv)

    if not args.dehyphen.exists():
        print(f"dehyphen.py not found at {args.dehyphen}", file=sys.stderr)
        return 1

    repos = list(args.repos) or []
    if args.all or not repos:
        repos = [pathlib.Path(r) for r in DEFAULT_REPOS]
    # Resolve and dedupe.
    repos = sorted({r.resolve() for r in repos if r.exists()})

    if not repos:
        print("No repos to sweep.", file=sys.stderr)
        return 1

    print(f"dehyphen sweep ({'APPLY' if args.apply else 'CHECK'} mode)")
    print(f"  script: {args.dehyphen}")
    print(f"  repos:  {len(repos)}")
    print()

    grand_total = 0
    bad = 0
    for repo in repos:
        print(f"== {repo} ==")
        summary = sweep_repo(repo, args.dehyphen, args.apply, push=not args.no_push)
        if summary.get("skipped"):
            print(f"  skipped: {summary['skipped']}")
            continue
        if args.apply:
            print(
                f"  swept={summary['swept']} "
                f"skipped_dirty={summary['skipped_dirty']} "
                f"failed_commit={summary['failed_commit']} "
                f"pushed={summary['pushed']}"
            )
        else:
            print(
                f"  violating files: {summary['files_violating']}; "
                f"total prose violations: {summary['violations']}"
            )
            if summary["skipped_dirty"]:
                print(f"  (skipped {summary['skipped_dirty']} dirty files)")
            if summary["violations"] > 0:
                bad += 1
        grand_total += summary.get("violations", 0)
        print()

    print(f"grand total: {grand_total} violations across {len(repos)} repos")
    return 1 if (not args.apply and bad > 0) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
