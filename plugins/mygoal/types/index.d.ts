// The finish check: a command whose exit code 0 means the goal is met,
// written by Claude, counted only once the operator approved it; `repo` is
// the git repo whose commits it tests ('' for the session directory).
export type Check = { argv: string[]; why: string; repo: string }

// What the judge (a separate Opus 5.5 call) said of the last check.
export type Verdict = { met: boolean; missing: string; outsideGoal: string }

// Where the goal stands.
//   none      no goal set
//   needs     goal set, no check proposed yet
//   approval  a check is proposed, waiting for the operator
//   working   check approved; Claude works, each new commit is checked
//   done      the check passed and the judge agreed
//   stopped   the operator stopped it
export type Status = 'none' | 'needs' | 'approval' | 'working' | 'done' | 'stopped'

// One run of the check and the judge, for the pane: which run it was, the
// commit it ran on, and what came of it.
export type Run = { check: number; commit: string; exitCode: number; tail: string; verdict: Verdict | null }

// Another session this one works with: when it was last heard from (its
// message or idle notice) and last given a check-up turn, the start of what
// it last said, and the start of what it was last asked.
export type Agent = { heardAt: number; checkedAt: number; said: string; asked: string }

declare module 'claude-code' {
  interface PluginState {
    mygoal: {
      // The sessions this one works with, by name
      agents: Record<string, Agent>
      // The time the pane counts from, moved once a minute
      now: number
      // The goal in the operator's words, as typed after /mygoal
      text: string
      status: Status
      proposed: Check | null
      approved: Check | null
      // How many times the check has run since approval
      checks: number
      last: Run | null
      // The commit the check last ran on ('' for none yet)
      checked: string
      // The last thing Claude was told, by commit: '<commit>' a failed
      // check, '<commit> done' a done claim sent back, '<commit> unreachable'
      told: string
      // Why it stopped, when it did
      note: string
    }
  }
}
