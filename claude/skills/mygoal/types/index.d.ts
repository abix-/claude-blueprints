// The finish check: a command whose exit code 0 means the goal is met,
// written by Claude, counted only once the operator approved it.
export type Check = { argv: string[]; why: string }

// What the judge (a separate Opus 5.5 call) said of the last check.
export type Verdict = { met: boolean; missing: string; outsideGoal: string }

// Where the goal stands.
//   none      no goal set
//   needs     goal set, no check proposed yet
//   approval  a check is proposed, waiting for the operator
//   working   check approved; Claude works, every turn is checked
//   done      the check passed and the judge agreed
//   stopped   the operator stopped it, or the turn limit was reached
export type Status = 'none' | 'needs' | 'approval' | 'working' | 'done' | 'stopped'

// One run of the check and the judge, for the pane.
export type Run = { turn: number; exitCode: number; tail: string; verdict: Verdict | null }

declare module 'claude-code' {
  interface PluginState {
    mygoal: {
      // The goal in the operator's words, as typed after /mygoal
      text: string
      status: Status
      proposed: Check | null
      approved: Check | null
      // Turns checked since the check was approved, and the limit
      turns: number
      limit: number
      last: Run | null
      // Why it stopped, when it did
      note: string
    }
  }
}
