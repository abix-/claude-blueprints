#!/usr/bin/env bash
# compare ~/.claude/skills/ against claude-blueprints/skills/
# shows which side is newer for each differing file

LIVE="$HOME/.claude/skills"
REPO="$(cd "$(dirname "$0")" && pwd)/skills"

red='\033[0;31m'
green='\033[0;32m'
yellow='\033[0;33m'
reset='\033[0m'

# find all SKILL.md files in both dirs
all_skills=$(
    (find "$LIVE" -name "SKILL.md" -not -path "*/.system/*" 2>/dev/null | sed "s|$LIVE/||"
     find "$REPO" -name "SKILL.md" 2>/dev/null | sed "s|$REPO/||") | sort -u
)

printf "%-40s %-12s %s\n" "SKILL" "STATUS" "DETAIL"
printf "%-40s %-12s %s\n" "-----" "------" "------"

for rel in $all_skills; do
    live_file="$LIVE/$rel"
    repo_file="$REPO/$rel"
    skill=$(dirname "$rel")

    if [ ! -f "$repo_file" ]; then
        printf "${yellow}%-40s %-12s %s${reset}\n" "$skill" "LIVE-ONLY" "not in repo"
        continue
    fi

    if [ ! -f "$live_file" ]; then
        printf "${yellow}%-40s %-12s %s${reset}\n" "$skill" "REPO-ONLY" "not in ~/.claude"
        continue
    fi

    if diff -q "$live_file" "$repo_file" > /dev/null 2>&1; then
        printf "%-40s %-12s %s\n" "$skill" "OK" "identical"
        continue
    fi

    # files differ -- check which is newer
    live_ts=$(stat -c %Y "$live_file" 2>/dev/null || stat -f %m "$live_file" 2>/dev/null)
    repo_ts=$(stat -c %Y "$repo_file" 2>/dev/null || stat -f %m "$repo_file" 2>/dev/null)

    live_date=$(date -d "@$live_ts" "+%Y-%m-%d %H:%M" 2>/dev/null || date -r "$live_ts" "+%Y-%m-%d %H:%M" 2>/dev/null)
    repo_date=$(date -d "@$repo_ts" "+%Y-%m-%d %H:%M" 2>/dev/null || date -r "$repo_ts" "+%Y-%m-%d %H:%M" 2>/dev/null)

    if [ "$live_ts" -gt "$repo_ts" ]; then
        printf "${green}%-40s %-12s %s${reset}\n" "$skill" "LIVE-NEWER" "live=$live_date repo=$repo_date"
    elif [ "$repo_ts" -gt "$live_ts" ]; then
        printf "${red}%-40s %-12s %s${reset}\n" "$skill" "REPO-NEWER" "live=$live_date repo=$repo_date"
    else
        printf "${yellow}%-40s %-12s %s${reset}\n" "$skill" "DIFF" "same mtime, content differs"
    fi
done
