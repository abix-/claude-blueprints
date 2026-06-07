#!/usr/bin/env bash
# Walk each differing SKILL.md between ~/.claude/skills and claude-blueprints/skills,
# show the diff, prompt per-file for an action.
#
# Actions:
#   l) copy LOCAL -> repo  (promote local edits)
#   r) copy REPO  -> local (accept repo version)
#   d) re-show diff
#   v) view side-by-side (less)
#   s) skip this file (decide later)
#   q) quit
#
# CRLF differences alone are not surfaced (normalized for comparison).

set -uo pipefail

LIVE="$HOME/.claude/skills"
REPO="$(cd "$(dirname "$0")" && pwd)/skills"

bold='\033[1m'
red='\033[0;31m'
green='\033[0;32m'
yellow='\033[0;33m'
cyan='\033[0;36m'
reset='\033[0m'

# Gather union of skill names
mapfile -t skills < <(
  (ls -1 "$LIVE" 2>/dev/null; ls -1 "$REPO" 2>/dev/null) \
    | sort -u
)

# Pre-classify
declare -a diff_list local_only repo_only
for s in "${skills[@]}"; do
  l="$LIVE/$s/SKILL.md"
  r="$REPO/$s/SKILL.md"
  if [ -f "$l" ] && [ -f "$r" ]; then
    if ! diff -q --strip-trailing-cr "$l" "$r" > /dev/null 2>&1; then
      diff_list+=("$s")
    fi
  elif [ -f "$l" ]; then
    local_only+=("$s")
  elif [ -f "$r" ]; then
    repo_only+=("$s")
  fi
done

echo
printf "${bold}Summary${reset}\n"
printf "  differ (content):   %d\n" "${#diff_list[@]}"
printf "  local only:         %d   (%s)\n" "${#local_only[@]}" "${local_only[*]:-}"
printf "  repo only:          %d   (%s)\n" "${#repo_only[@]}" "${repo_only[*]:-}"
echo

total=${#diff_list[@]}
i=0
for s in "${diff_list[@]}"; do
  i=$((i+1))
  l="$LIVE/$s/SKILL.md"
  r="$REPO/$s/SKILL.md"
  lv=$(grep -m1 '^version:' "$l" | sed 's/version: *//;s/"//g;s/\r//g')
  rv=$(grep -m1 '^version:' "$r" | sed 's/version: *//;s/"//g;s/\r//g')
  ll=$(wc -l < "$l")
  rl=$(wc -l < "$r")
  only_local=$(diff --strip-trailing-cr "$l" "$r" | grep -c '^<')
  only_repo=$(diff --strip-trailing-cr "$l" "$r" | grep -c '^>')

  while true; do
    echo
    printf "${bold}[%d/%d] %s${reset}\n" "$i" "$total" "$s"
    printf "  local: v%-6s %4d lines  (-local: %d unique lines)\n" "${lv:-?}" "$ll" "$only_local"
    printf "  repo:  v%-6s %4d lines  (+repo:  %d unique lines)\n" "${rv:-?}" "$rl" "$only_repo"
    printf "  ${cyan}[l]${reset} local->repo  ${cyan}[r]${reset} repo->local  ${cyan}[d]${reset} diff  ${cyan}[v]${reset} side-by-side  ${cyan}[s]${reset} skip  ${cyan}[q]${reset} quit\n"
    printf "  action> "
    read -r action </dev/tty || { echo; exit 0; }
    case "$action" in
      l)
        cp "$l" "$r"
        printf "  ${green}copied local -> repo${reset}\n"
        break
        ;;
      r)
        cp "$r" "$l"
        printf "  ${green}copied repo -> local${reset}\n"
        break
        ;;
      d)
        diff --strip-trailing-cr -u "$l" "$r" | sed 's/^/    /' | head -200
        ;;
      v)
        diff --strip-trailing-cr -y --width=200 "$l" "$r" | less -R
        ;;
      s)
        printf "  ${yellow}skipped${reset}\n"
        break
        ;;
      q)
        echo "quit"
        exit 0
        ;;
      *)
        printf "  unknown: %s\n" "$action"
        ;;
    esac
  done
done

echo
printf "${bold}Done with content diffs.${reset}\n"

if [ "${#local_only[@]}" -gt 0 ]; then
  echo
  printf "${bold}Local-only skills (decide whether to push to repo):${reset}\n"
  for s in "${local_only[@]}"; do
    printf "  - %s   (%s)\n" "$s" "$LIVE/$s/SKILL.md"
  done
fi

if [ "${#repo_only[@]}" -gt 0 ]; then
  echo
  printf "${bold}Repo-only skills (decide whether to install locally):${reset}\n"
  for s in "${repo_only[@]}"; do
    printf "  - %s   (%s)\n" "$s" "$REPO/$s/SKILL.md"
  done
fi
