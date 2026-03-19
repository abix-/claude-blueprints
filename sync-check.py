"""Compare ~/.claude/skills/ against claude-blueprints/skills/, optionally sync."""
import argparse
import hashlib
import os
import shutil
import sys
from datetime import datetime
from pathlib import Path

LIVE = Path(os.environ.get("USERPROFILE", os.environ.get("HOME", ""))) / ".claude" / "skills"
REPO = Path(__file__).resolve().parent / "skills"


def find_skills(root):
    skills = {}
    for p in root.rglob("SKILL.md"):
        if ".system" in p.parts:
            continue
        rel = p.relative_to(root)
        skill = str(rel.parent).replace("\\", "/")
        skills[skill] = p
    return skills


def md5(path):
    return hashlib.md5(path.read_bytes()).hexdigest()[:8]


def check(live_skills, repo_skills):
    all_names = sorted(set(live_skills) | set(repo_skills))
    rows = []
    for name in all_names:
        lf = live_skills.get(name)
        rf = repo_skills.get(name)

        if not rf:
            rows.append((name, "LOCAL-ONLY", "not in repo", lf, None))
            continue
        if not lf:
            rows.append((name, "GIT-ONLY", "not in ~/.claude", None, rf))
            continue

        lbytes = lf.read_bytes()
        rbytes = rf.read_bytes()
        lmd5 = hashlib.md5(lbytes).hexdigest()[:8]
        rmd5 = hashlib.md5(rbytes).hexdigest()[:8]

        if lbytes == rbytes:
            rows.append((name, "OK", f"identical  {lmd5}", lf, rf))
            continue

        lt = lf.stat().st_mtime
        rt = rf.stat().st_mtime
        ld = datetime.fromtimestamp(lt).strftime("%Y-%m-%d %H:%M")
        rd = datetime.fromtimestamp(rt).strftime("%Y-%m-%d %H:%M")

        hashes = f"live={lmd5} repo={rmd5}"
        if lt > rt:
            rows.append((name, "LOCAL-AHEAD", f"{hashes}  live={ld}  repo={rd}", lf, rf))
        elif rt > lt:
            rows.append((name, "GIT-AHEAD", f"{hashes}  live={ld}  repo={rd}", lf, rf))
        else:
            rows.append((name, "DIFF", f"{hashes}  same mtime", lf, rf))
    return rows


def print_table(rows):
    print(f"{'SKILL':<40} {'STATUS':<12} DETAIL")
    print(f"{'-----':<40} {'------':<12} ------")
    for name, status, detail, _, _ in rows:
        print(f"{name:<40} {status:<12} {detail}")


def sync(rows, direction):
    """Copy files to resolve drift. direction: 'push' (local->repo) or 'pull' (repo->local)."""
    copied = 0
    for name, status, _, lf, rf in rows:
        if status == "OK":
            continue

        if direction == "push":
            if status in ("LOCAL-AHEAD", "LOCAL-ONLY", "DIFF"):
                dst = REPO / name / "SKILL.md"
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(lf, dst)
                print(f"  {name}: local -> repo")
                copied += 1
            elif status == "GIT-ONLY":
                # exists in repo but not local -- skip on push
                pass
        elif direction == "pull":
            if status in ("GIT-AHEAD", "GIT-ONLY", "DIFF"):
                dst = LIVE / name / "SKILL.md"
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(rf, dst)
                print(f"  {name}: repo -> local")
                copied += 1
            elif status == "LOCAL-ONLY":
                # exists locally but not in repo -- skip on pull
                pass

    return copied


def main():
    parser = argparse.ArgumentParser(description="sync ~/.claude/skills with claude-blueprints")
    parser.add_argument("action", nargs="?", default="check",
                        choices=["check", "push", "pull"],
                        help="check: show drift. push: copy local->repo. pull: copy repo->local.")
    args = parser.parse_args()

    live_skills = find_skills(LIVE)
    repo_skills = find_skills(REPO)
    rows = check(live_skills, repo_skills)

    print_table(rows)

    drifted = [r for r in rows if r[1] != "OK"]
    if not drifted:
        print(f"\nall {len(rows)} skills in sync")
        return

    print(f"\n{len(drifted)} skill(s) out of sync")

    if args.action == "check":
        sys.exit(1)

    print(f"\nsyncing ({args.action})...")
    copied = sync(rows, args.action)
    print(f"\n{copied} file(s) copied")


if __name__ == "__main__":
    main()
