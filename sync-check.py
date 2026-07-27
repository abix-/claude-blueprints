"""Install and compare shared skills for Claude and Codex.

Usage:
    python sync-check.py check [--runtime claude|codex|all]
    python sync-check.py install [--runtime claude|codex|all]
"""

import argparse
import os
import shutil
import sys
from pathlib import Path
from typing import NamedTuple


REPO = Path(__file__).resolve().parent
DEFAULT_HOME = Path(os.environ.get("USERPROFILE", os.environ.get("HOME", "")))
RUNTIMES = ("claude", "codex")


class Drift(NamedTuple):
    source: Path
    destination: Path
    status: str


def iter_files(root):
    if not root.is_dir():
        return
    for path in sorted(root.rglob("*")):
        if path.is_file():
            yield path


def add_tree(manifest, source_root, destination_root):
    for source in iter_files(source_root):
        relative = source.relative_to(source_root)
        destination = destination_root / relative
        existing = manifest.get(destination)
        if existing is not None:
            raise ValueError(
                f"duplicate destination: {destination} from "
                f"{existing} and {source}"
            )
        manifest[destination] = source


def build_manifest(repo, home, runtime):
    repo = Path(repo)
    home = Path(home)
    if runtime not in RUNTIMES:
        raise ValueError(f"unknown runtime: {runtime}")

    shared_skills = repo / "skills"
    runtime_root = repo / runtime
    runtime_skills = runtime_root / "skills"
    shared_names = {
        path.name
        for path in shared_skills.iterdir()
        if path.is_dir() and (path / "SKILL.md").is_file()
    }
    runtime_names = (
        {
            path.name
            for path in runtime_skills.iterdir()
            if path.is_dir() and (path / "SKILL.md").is_file()
        }
        if runtime_skills.is_dir()
        else set()
    )
    conflicts = sorted(shared_names & runtime_names)
    if conflicts:
        raise ValueError(
            "runtime skill conflicts with shared skill: "
            + ", ".join(conflicts)
        )

    if runtime == "claude":
        skill_destination = home / ".claude" / "skills"
        config_destination = home / ".claude"
    else:
        skill_destination = home / ".agents" / "skills"
        config_destination = home / ".codex"

    manifest = {}
    add_tree(manifest, shared_skills, skill_destination)
    add_tree(manifest, runtime_skills, skill_destination)

    for source in iter_files(runtime_root):
        relative = source.relative_to(runtime_root)
        if relative.parts[0] == "skills":
            continue
        destination = config_destination / relative
        manifest[destination] = source

    return manifest


def check(repo=REPO, home=DEFAULT_HOME, runtime="claude"):
    rows = []
    for destination, source in sorted(
        build_manifest(repo, home, runtime).items(),
        key=lambda item: str(item[0]),
    ):
        if not destination.is_file():
            status = "MISSING"
        elif source.read_bytes() == destination.read_bytes():
            status = "OK"
        else:
            status = "CHANGED"
        rows.append(Drift(source, destination, status))
    return rows


def install(repo=REPO, home=DEFAULT_HOME, runtime="claude"):
    copied = 0
    for destination, source in build_manifest(repo, home, runtime).items():
        if destination.is_file() and source.read_bytes() == destination.read_bytes():
            continue
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
        copied += 1
    return copied


def print_rows(runtime, rows, home):
    print(f"\n{runtime}")
    for row in rows:
        relative = row.destination.relative_to(home)
        print(f"{row.status:<8} {relative}")


def selected_runtimes(value):
    if value == "all":
        return RUNTIMES
    return (value,)


def main():
    parser = argparse.ArgumentParser(
        description="install and compare shared Claude and Codex blueprints"
    )
    parser.add_argument(
        "action",
        nargs="?",
        default="check",
        choices=["check", "install"],
    )
    parser.add_argument(
        "--runtime",
        choices=["claude", "codex", "all"],
        default="all",
    )
    parser.add_argument(
        "--home",
        type=Path,
        default=DEFAULT_HOME,
    )
    args = parser.parse_args()

    failed = False
    for runtime in selected_runtimes(args.runtime):
        if args.action == "install":
            copied = install(REPO, args.home, runtime)
            print(f"{runtime}: copied {copied} file(s)")

        rows = check(REPO, args.home, runtime)
        print_rows(runtime, rows, args.home)
        if any(row.status != "OK" for row in rows):
            failed = True

    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
