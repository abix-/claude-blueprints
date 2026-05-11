#!/usr/bin/env bash
# install-git-hooks.sh: drop the dehyphen pre-commit hook into one or
# more repos.
#
# Usage:
#   bash scripts/install-git-hooks.sh /path/to/repo [/path/to/another ...]
#   bash scripts/install-git-hooks.sh --all          # install into known owned repos
#   bash scripts/install-git-hooks.sh --uninstall <repo>  # remove the hook
#
# The hook is copied (not symlinked) so each repo has a standalone
# copy. Re-run the installer after script updates to refresh the
# copies, or pass --symlink to symlink instead (Windows requires
# admin or developer mode for symlinks).

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOK_SRC="$SCRIPT_DIR/git-hooks/pre-commit"

if [ ! -f "$HOOK_SRC" ]; then
    echo "install-git-hooks: source hook not found at $HOOK_SRC" >&2
    exit 1
fi

# Known owned repos for --all. Edit to taste.
ALL_REPOS=(
    "/c/code/claude-blueprints"
    "/c/code/grounded2mods"
    "/c/code/abixio"
    "/c/code/abixio-ui"
    "/c/code/endless"
    "/c/code/k3sc"
    "/c/code/Schedule1Mods"
    "/c/code/chromium-extensions"
)

mode="install"
symlink=0
targets=()

while [ $# -gt 0 ]; do
    case "$1" in
        --all) targets=("${ALL_REPOS[@]}"); shift ;;
        --uninstall) mode="uninstall"; shift ;;
        --symlink) symlink=1; shift ;;
        --help|-h)
            sed -n '2,12p' "$0"
            exit 0
            ;;
        *) targets+=("$1"); shift ;;
    esac
done

if [ ${#targets[@]} -eq 0 ]; then
    echo "install-git-hooks: no target repos given" >&2
    echo "Usage: $0 <repo> [repo ...] | --all" >&2
    exit 1
fi

for repo in "${targets[@]}"; do
    if [ ! -d "$repo/.git" ] && [ ! -f "$repo/.git" ]; then
        echo "skip: $repo (not a git repo)"
        continue
    fi
    hook_dir="$repo/.git/hooks"
    # Worktrees: .git is a file pointing at the real gitdir.
    if [ -f "$repo/.git" ]; then
        gitdir=$(sed -n 's/^gitdir: //p' "$repo/.git")
        hook_dir="$gitdir/hooks"
    fi
    mkdir -p "$hook_dir"
    dest="$hook_dir/pre-commit"

    case "$mode" in
        uninstall)
            if [ -e "$dest" ] || [ -L "$dest" ]; then
                rm -f "$dest"
                echo "uninstalled: $dest"
            else
                echo "no hook at: $dest"
            fi
            ;;
        install)
            if [ "$symlink" -eq 1 ]; then
                ln -sf "$HOOK_SRC" "$dest"
                echo "linked: $dest -> $HOOK_SRC"
            else
                cp "$HOOK_SRC" "$dest"
                chmod +x "$dest"
                echo "installed: $dest"
            fi
            ;;
    esac
done
