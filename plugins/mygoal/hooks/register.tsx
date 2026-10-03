import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { Check, Run, Status, Verdict } from '../types'

// /mygoal: keeps Claude on the operator's goal until it is truly done.
//
// Claude does not decide "done". Two things outside Claude do:
//   the finish check: a command Claude proposes and the operator approves
//     with a button only the operator can press; this mod runs it itself,
//     once for each new commit in the check's repo, and reads the exit code,
//   the judge: a separate call to Opus 5.5 that sees only the goal in the
//     operator's words, the check's real output and the code changes since
//     approval, never Claude's explanation. It says what is missing and
//     what was done that the goal did not ask for.
// The check runs when the work changes, as CI runs on a commit (Rust's
// bors), never because a turn ended: a turn on the same commit costs
// nothing and is told nothing, unless it claims the goal done. A failed
// check is told once, with what changed since the check before.
//
// The stock /goal is left alone; this is a separate command to compare.

const PANE = 'mygoal'
const TOOL = 'propose_check'
const JUDGE_MODEL = 'claude-opus-5-5'
const TAIL_CHARS = 4000
const CHANGES_CHARS = 40000

// A check whose output says it could not reach what it tests (the game not
// running): not a test result, so the commit stays unchecked and the check
// runs again after a later turn, told once.
const UNREACHABLE = /no .{0,60} answering|connection refused|could not connect/i
// A turn that claims the goal done.
const CLAIMS_DONE = /\b(goal (is )?(done|met|complete)|all (\d+ )?(\w+ )?tests? pass(ed)?)\b/i
// The lines of a check's output that say how a test went.
const RESULT_LINE = /\b(pass(ed)?|fail(ed)?)\b/i

const goalText = atom({ plugin: 'mygoal', key: 'text' } as const, '')
const status = atom({ plugin: 'mygoal', key: 'status' } as const, 'none')
const proposed = atom({ plugin: 'mygoal', key: 'proposed' } as const, null)
const approved = atom({ plugin: 'mygoal', key: 'approved' } as const, null)
// How many times the check has run since approval
const checks = atom({ plugin: 'mygoal', key: 'checks' } as const, 0)
const last = atom({ plugin: 'mygoal', key: 'last' } as const, null)
// The commit the check last ran on, and the last commit Claude was told about
const checked = atom({ plugin: 'mygoal', key: 'checked' } as const, '')
const told = atom({ plugin: 'mygoal', key: 'told' } as const, '')
const note = atom({ plugin: 'mygoal', key: 'note' } as const, '')

// One check-and-judge at a time.
let busy = false

// The other sessions this one works with, by name, and when each was last
// heard from (its message or its idle notice) or last checked on. While a
// goal is working, one silent for CHECK_IN_MS gets a check-up turn.
const CHECK_IN_MS = 10 * 60 * 1000
const CHECK_EVERY_MS = 60 * 1000
const SAID_CHARS = 160
const agents = atom({ plugin: 'mygoal', key: 'agents' } as const, {})
const now = atom({ plugin: 'mygoal', key: 'now' } as const, 0)

// The start of a message, on one line, without its envelope's tags.
const gist = (text: string) => {
  const flat = text.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
  return flat.length > SAID_CHARS ? `${flat.slice(0, SAID_CHARS)}...` : flat
}

// What a delivery says of each session it is from: a peer's message
// (from-name="x", the message itself) or its idle notice ("x", which you
// asked ...: idle).
function heardIn(text: string): [string, string][] {
  const out: [string, string][] = []
  for (const m of text.matchAll(/from-name="([^"]+)"[^>]*>([\s\S]*?)(<\/cross-session-message>|$)/g)) out.push([m[1], gist(m[2])])
  for (const m of text.matchAll(/idle notice\] "([^"]+)"[^\n]*?finished a turn at (\d+:\d+)/g)) out.push([m[1], `idle since ${m[2]}`])
  return out
}

async function checkIn($: EngineInterface) {
  const t = await $.clock.now()
  await update($, now, () => t)
  if ((await read($, status)) !== 'working') return
  const all = await read($, agents)
  const silent = Object.keys(all).filter(name => t - Math.max(all[name].heardAt, all[name].checkedAt) >= CHECK_IN_MS)
  if (silent.length === 0) return
  await update($, agents, a => Object.fromEntries(Object.entries(a).map(([n, x]) => [n, silent.includes(n) ? { ...x, checkedAt: t } : x])))
  await $.prompt.submit({
    text: [
      `mygoal: no word from ${silent.join(', ')} in ${CHECK_IN_MS / 60000} minutes.`,
      'Check on each now: read its repo (git log, git status, its logs), then ask it for its status with SendMessage.',
      'If it waits on a decision, make it yourself from the docs and tell it to carry on; never leave it waiting on the operator.',
    ].join('\n'),
  })
}

const JUDGE = [
  'You judge whether an operator\'s goal is met. You see only the goal, in the operator\'s own words,',
  'the finish check the operator approved with its real output, and the code changes made for it.',
  'You do not see the worker\'s explanation, and nothing it could say changes the evidence.',
  'Read the goal\'s words in their plain everyday meaning. Do not accept a narrower reading of any word.',
  'If any part of the goal is not shown done by the evidence, the goal is not met.',
  'Also list everything in the code changes that the goal did not ask for.',
  'Answer with JSON only, no other text:',
  '{"met": true or false, "missing": "what is not done yet, or empty", "outside_goal": "changes the goal did not ask for, or empty"}',
].join('\n')

// How to propose the finish check: in the tool's description, read at every
// call, and in the request while a check is needed.
const PROPOSING = [
  'The check is half of "done", so propose the one hardest to pass falsely.',
  'It must fail while any part of the goal is unmet, in the plain meaning of the operator\'s words: if it could exit 0 while the goal is not done, it is the wrong check.',
  'Cover the whole goal, not the easy part. Run against the real thing (the live game, the real CLI, the deploy script), never a mock that cannot see the failure.',
  'argv runs with no shell from the session directory; wrap a pipeline in a script that is part of the work. why says in one or two sentences why exit 0 proves the whole goal.',
  'For a game with a test queue: write each piece of the goal as its own test first, see it fail, commit the tests before the goal starts, and check with the queue run of those tests (for topside, pwsh -NoProfile -File scripts/build.ps1 queue <test files>).',
  'A goal no command can prove leaves only the judge: say so in why rather than propose a check that proves something smaller.',
].join('\n')

// What never happens while a goal is open, in every request.
const NEVER = [
  'Never propose a check that proves a narrower goal than the operator wrote.',
  'Never weaken the approved check by proposing a new one mid-goal to get past a failure; a new proposal pauses the goal until the operator approves it.',
  'Never treat a "not done yet" turn as a suggestion: work on what it lists as missing, undo what it lists as outside the goal.',
  'Never touch the goal store or this mod while a goal is open; those tool calls are refused.',
  'Never run /mygoal and the stock /goal in one session; both start the next turn and compete.',
].join('\n')

// Is the goal one that keeps Claude working?
const isOpen = (s: Status) => s === 'needs' || s === 'approval' || s === 'working'

// Save everything a later session needs to carry on.
async function persist($: EngineInterface) {
  await $.store.set('goal', {
    text: await read($, goalText),
    status: await read($, status),
    proposed: await read($, proposed),
    approved: await read($, approved),
    checks: await read($, checks),
    last: await read($, last),
    checked: await read($, checked),
    told: await read($, told),
    note: await read($, note),
  })
}

async function load($: EngineInterface) {
  const saved = (await $.store.get('goal')) as
    | { text: string; status: Status; proposed: Check | null; approved: Check | null; checks?: number; last: Run | null; checked?: string; told?: string; note: string }
    | undefined
  if (!saved) return
  await update($, goalText, () => saved.text)
  await update($, status, () => saved.status)
  await update($, proposed, () => saved.proposed)
  await update($, approved, () => saved.approved)
  await update($, checks, () => saved.checks ?? 0)
  await update($, last, () => saved.last)
  await update($, checked, () => saved.checked ?? '')
  await update($, told, () => saved.told ?? '')
  await update($, note, () => saved.note)
}

// The repo the check tests, as `git -C` takes it; the session directory
// when the check named none.
const repoOf = (check: Check) => check.repo || '.'

// The commit the repo stands at now, or '' when it is not a git repo.
async function headOf($: EngineInterface, repo: string): Promise<string> {
  const r = await $.process.run(['git', '-C', repo, 'rev-parse', 'HEAD'])
  return r.exitCode === 0 ? r.stdout.trim() : ''
}

// The working tree against the commit at approval (`base`), split per file,
// plus the untracked files: commits since approval and edits not yet
// committed alike, so the judge sees everything done for the goal.
async function treeNow($: EngineInterface, repo: string, base: string): Promise<Record<string, string>> {
  const out: Record<string, string> = {}
  const diff = await $.process.run(['git', '-C', repo, 'diff', base || 'HEAD'])
  if (diff.exitCode !== 0) return out
  for (const part of diff.stdout.split(/^(?=diff --git )/m)) {
    const name = /^diff --git a\/(.+?) b\//.exec(part)?.[1]
    if (name) out[name] = part
  }
  const st = await $.process.run(['git', '-C', repo, 'status', '--porcelain'])
  for (const line of st.stdout.split('\n')) {
    if (line.startsWith('?? ')) out[line.slice(3).trim()] = '(new file, not tracked yet)'
  }
  return out
}

// What changed since approval: every file whose diff from the commit at
// approval differs from what it was at approval (edits already in the tree
// then are left out until they change again).
async function changesSinceApproval($: EngineInterface, repo: string): Promise<string> {
  const before = ((await $.store.get('baseline')) ?? {}) as Record<string, string>
  const base = ((await $.store.get('base')) ?? '') as string
  const now = await treeNow($, repo, base)
  const parts = Object.entries(now)
    .filter(([file, text]) => before[file] !== text)
    .map(([file, text]) => (text.startsWith('diff --git') ? text : `${file}: ${text}`))
  const all = parts.join('\n')
  return all.length > CHANGES_CHARS ? `${all.slice(0, CHANGES_CHARS)}\n(cut at ${CHANGES_CHARS} characters)` : all
}

async function judge($: EngineInterface, goal: string, check: Check, exitCode: number, tail: string, changes: string): Promise<Verdict | null> {
  const r = await $.model.complete({
    model: JUDGE_MODEL,
    system: JUDGE,
    prompt: [
      'GOAL, in the operator\'s words:',
      goal,
      '',
      `FINISH CHECK the operator approved: ${check.argv.join(' ')}`,
      `Why it proves the goal: ${check.why}`,
      `Exit code: ${exitCode}`,
      'Last part of its output:',
      tail || '(no output)',
      '',
      'CODE CHANGES since the check was approved:',
      changes || '(none)',
    ].join('\n'),
    maxTokens: 800,
    effort: 'high',
    timeoutMs: 120000,
  })
  if (!r.isAnswered) return null
  const m = /\{[\s\S]*\}/.exec(r.text)
  if (!m) return null
  try {
    const v = JSON.parse(m[0]) as { met?: unknown; missing?: unknown; outside_goal?: unknown }
    return { met: v.met === true, missing: String(v.missing ?? ''), outsideGoal: String(v.outside_goal ?? '') }
  } catch {
    return null
  }
}

// The lines of `tail` that say how a test went and are not in `before`:
// what changed since the check before.
function changedLines(tail: string, before: string): string {
  const was = new Set(before.split('\n').map(l => l.trim()))
  return tail
    .split('\n')
    .map(l => l.trim())
    .filter(l => RESULT_LINE.test(l) && !was.has(l))
    .join('\n')
}

// The "not done yet" message: the goal, the check's last result, the judge.
function notDone(goal: string, check: Check, run: Run, before: Run | null, why: string): string {
  const changed = before ? changedLines(run.tail, before.tail) : ''
  return [
    `mygoal: the goal is not done yet. ${why}`,
    'The goal, in the operator\'s words:',
    goal,
    '',
    `The finish check \`${check.argv.join(' ')}\` exited ${run.exitCode} on commit ${run.commit.slice(0, 8)}. Last part of its output:`,
    run.tail || '(no output)',
    '',
    before ? `Changed since the check before (on ${before.commit.slice(0, 8)}):\n${changed || '(no test changed)'}` : '',
    run.verdict
      ? `The judge (a separate Opus 5.5 call that sees only the goal, this output and the code changes) says\nmissing: ${run.verdict.missing || '(nothing)'}\noutside the goal: ${run.verdict.outsideGoal || '(nothing)'}`
      : 'The judge did not answer this time.',
    '',
    'Keep working on the goal. Undo anything outside the goal. The check runs again on the next commit; only it passing and the judge agreeing end this.',
  ].join('\n')
}

// After a working turn (`answer`, what Claude said): when the check's repo
// has a commit the check has not run on, run it once, ask the judge, then
// finish or tell Claude what is missing, once. On a commit already checked,
// nothing runs and nothing is said, unless the turn claims the goal done.
async function evaluate($: EngineInterface, answer: string) {
  const check = await read($, approved)
  if (!check) return
  const repo = repoOf(check)
  const head = await headOf($, repo)
  const goal = await read($, goalText)
  const before = await read($, last)

  if (head === (await read($, checked))) {
    if (before && CLAIMS_DONE.test(answer) && (await read($, told)) !== `${head} done`) {
      await update($, told, () => `${head} done`)
      await persist($)
      await $.prompt.submit({ text: notDone(goal, check, before, null, 'Nothing has been committed since the check last ran, so it has not passed on this commit.') })
    }
    return
  }

  let exitCode = -1
  let tail = ''
  try {
    const r = await $.process.run(check.argv, { timeoutMs: 600000 })
    exitCode = r.exitCode
    const both = `${r.stdout}${r.stderr ? `\n${r.stderr}` : ''}`
    tail = both.length > TAIL_CHARS ? both.slice(-TAIL_CHARS) : both
  } catch (err) {
    tail = `the check could not run: ${String(err)}`
  }
  const n = (await read($, checks)) + 1
  await update($, checks, () => n)

  // Not a test result: the commit stays unchecked, so a later turn runs the
  // check again; Claude is told once.
  if (exitCode !== 0 && UNREACHABLE.test(tail)) {
    await update($, note, () => `check ${n} on ${head.slice(0, 8)} could not reach what it tests`)
    if ((await read($, told)) !== `${head} unreachable`) {
      await update($, told, () => `${head} unreachable`)
      await persist($)
      await $.prompt.submit({
        text: [
          `mygoal: the finish check could not reach what it tests (commit ${head.slice(0, 8)}), so this is not a test result. Last part of its output:`,
          tail || '(no output)',
          '',
          'Get it running (for a game, its writer launches it). The check runs again after a later turn, without telling you again until it reaches it.',
        ].join('\n'),
      })
    } else {
      await persist($)
    }
    return
  }

  await update($, checked, () => head)
  const changes = await changesSinceApproval($, repo)
  const verdict = await judge($, goal, check, exitCode, tail, changes)
  const run: Run = { check: n, commit: head, exitCode, tail, verdict }
  await update($, last, () => run)

  if (exitCode === 0 && verdict?.met && verdict.outsideGoal.trim() === '') {
    await update($, status, () => 'done')
    await update($, note, () => `done on check ${n}, commit ${head.slice(0, 8)}: the check exited 0 and the judge agreed`)
    await persist($)
    $.ui.toast('mygoal: done, checked and judged')
    return
  }
  await update($, told, () => head)
  await persist($)
  await $.prompt.submit({ text: notDone(goal, check, run, before, `(check ${n}, on a new commit)`) })
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'mygoal',
      description: 'Work on your goal until an approved check passes and a separate judge agrees',
      argumentHint: '<your goal> | stop',
      immediate: true,
    })
    await $.tool.register({
      name: TOOL,
      description:
        `Propose the finish check for the operator's /mygoal goal: a command whose exit code 0 proves the goal is met in the plain meaning of the operator's words. The operator approves or rejects it; until approved, do no goal work.\n${PROPOSING}`,
      inputSchema: {
        type: 'object',
        properties: {
          argv: { type: 'array', items: { type: 'string' }, description: 'The command and its arguments, run with no shell from the session directory' },
          why: { type: 'string', description: 'Why exit code 0 proves the whole goal' },
          repo: { type: 'string', description: 'The git repo whose commits the check tests (an absolute path); the check runs again each time it has a new commit' },
        },
        required: ['argv', 'why', 'repo'],
      },
    })
    await load($)
    $.clock.every(CHECK_EVERY_MS, () => void checkIn($))
    return next(e)
  })

  on('session.receive', async ($, e, next) => {
    const said = heardIn(e.text)
    if (said.length > 0) {
      const t = await $.clock.now()
      await update($, agents, a => {
        const out = { ...a }
        for (const [name, words] of said) {
          const was = out[name] ?? { heardAt: t, checkedAt: 0, said: '', asked: '' }
          // An idle notice says only that it stopped; keep what it last said.
          const before = was.said.replace(/ \(idle since [^)]*\)$/, '')
          out[name] = { ...was, heardAt: t, said: words.startsWith('idle since') && before !== '' && !before.startsWith('idle since') ? `${before} (${words})` : words }
        }
        return out
      })
    }
    return next(e)
  })

  // A session this one messages is one it works with: its silence counts
  // from the first message, until it answers.
  on('session.send', async ($, e, next) => {
    const t = await $.clock.now()
    await update($, agents, a => ({ ...a, [e.to]: { ...(a[e.to] ?? { heardAt: t, checkedAt: 0, said: '' }), asked: gist(e.text) } }))
    return next(e)
  })

  // Only the operator sets or stops the goal: a prompt typed at the composer.
  on('command.run', { command: 'mygoal' }, async ($, e) => {
    if (e.origin?.kind !== 'composer') return { text: 'mygoal: only the operator sets the goal' }
    const words = e.args.trim()
    if (words === 'stop') {
      await update($, status, () => 'stopped')
      await update($, note, () => 'stopped by the operator')
      await persist($)
      return { text: 'mygoal: stopped' }
    }
    if (words !== '') {
      await update($, goalText, () => words)
      await update($, status, () => 'needs')
      await update($, proposed, () => null)
      await update($, approved, () => null)
      await update($, checks, () => 0)
      await update($, last, () => null)
      await update($, checked, () => '')
      await update($, told, () => '')
      await update($, note, () => '')
      await persist($)
      // A command cannot start a turn while it runs; start it once it has.
      void $.clock.after(0, () => {
        void $.prompt.submit({
          text: 'mygoal: the operator set a new goal. Before any other work, propose its finish check with the propose_check tool.',
        })
      })
    }
    await $.ui.open({ id: PANE, title: 'mygoal' })
    return {}
  })

  on('tool.call', { tool: 'mcp__mygoal__propose_check' }, async ($, e) => {
    const s = await read($, status)
    if (!isOpen(s)) return { result: 'No /mygoal goal is active.' }
    type Input = { argv?: unknown; why?: unknown; repo?: unknown }
    const input = (e as unknown as { input?: Input }).input ?? (e as unknown as Input)
    const argv = Array.isArray(input.argv) ? input.argv.map(String) : []
    if (argv.length === 0) return { result: 'argv must be the command and its arguments, at least one item.' }
    const repo = String(input.repo ?? '')
    if ((await headOf($, repo || '.')) === '') return { result: `repo must be a git repo; \`git -C ${repo || '.'} rev-parse HEAD\` failed.` }
    await update($, proposed, () => ({ argv, why: String(input.why ?? ''), repo }))
    await update($, status, () => 'approval')
    await persist($)
    void $.ui.open({ id: PANE, title: 'mygoal' })
    return {
      result:
        'Proposed. The operator approves or rejects it in the mygoal pane. Stop here and wait for their answer; do not start the goal work until the check is approved.',
    }
  })

  // While a goal is open, nothing Claude runs may touch where the goal is
  // kept (the plugin store) or this mod's code: the repo's
  // plugins/mygoal, loaded in place through the claude-blueprints
  // marketplace, which Claude Code also keeps under
  // ~/.claude/plugins/cache/claude-blueprints/mygoal.
  on('tool.call', async ($, e, next) => {
    if (!isOpen(await read($, status)) || e.tool.startsWith('mcp__mygoal__')) return next(e)
    const asked = JSON.stringify(e)
    if (/plugins[\\/]+store/i.test(asked) || /(plugins|claude-blueprints)[\\/]+mygoal/i.test(asked)) {
      return { deny: 'mygoal: while a goal is open, the goal and the mygoal mod are not Claude\'s to change.' }
    }
    return next(e)
  })

  // The goal, word for word, in every request while it is open.
  on('prompt.compose', async ($, e, next) => {
    const r = await next(e)
    const s = await read($, status)
    if (!isOpen(s)) return r
    const step =
      s === 'needs'
        ? `Before any other work, propose the finish check with the propose_check tool: a command whose exit code 0 proves the goal is met in the plain meaning of the operator's words.\n${PROPOSING}`
        : s === 'approval'
          ? 'Your proposed finish check is waiting for the operator\'s approval. Do no goal work until it is approved.'
          : 'Work on this goal and nothing else. Do nothing the goal did not ask for. Read the docs first, then make the decisions yourself and keep the work moving; ask the operator only before the work starts, never wait on them mid-goal. The operator changes what they disagree with.'
    const text = [
      'The operator\'s goal (/mygoal), in their words:',
      await read($, goalText),
      '',
      'You do not decide when this goal is done. Each time the check\'s repo has a new commit, the approved finish check is run for you and a separate judge reads the goal from its output and your code changes alone. Until both agree on a commit, the work goes on. Turns with no new commit are not checked.',
      step,
      '',
      NEVER,
    ].join('\n')
    return { ...r, sections: [...r.sections, { id: 'mygoal:goal', text, scope: 'session' as const }] }
  })

  on('turn.complete', async ($, e, next) => {
    const r = await next(e)
    if (e.agentId || e.isAborted || busy) return r
    const s = await read($, status)
    if (s === 'working') {
      busy = true
      try {
        await evaluate($, e.answer ?? '')
      } finally {
        busy = false
      }
    }
    return r
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Text, Button, Code } = $.ui.resolve(e)
    const s = await read($, status)
    const goal = await read($, goalText)
    const prop = await read($, proposed)
    const check = await read($, approved)
    const n = await read($, checks)
    const run = await read($, last)
    const why = await read($, note)
    const team = Object.entries(await read($, agents)).sort(([a], [b]) => a.localeCompare(b))
    const t = await read($, now)
    const ago = (at: number) => `${Math.max(0, Math.floor((t - at) / 60000))} min ago`

    const approve = async () => {
      const p = await read($, proposed)
      if (!p) return
      const base = await headOf($, repoOf(p))
      await $.store.set('base', base)
      await $.store.set('baseline', await treeNow($, repoOf(p), base))
      await update($, approved, () => p)
      await update($, proposed, () => null)
      await update($, status, () => 'working')
      await update($, checks, () => 0)
      await update($, last, () => null)
      await update($, checked, () => '')
      await update($, told, () => '')
      await update($, note, () => '')
      await persist($)
      await $.prompt.submit({ text: `mygoal: the operator approved the finish check \`${p.argv.join(' ')}\`. Work on the goal now.` })
    }
    const reject = async () => {
      await update($, proposed, () => null)
      await update($, status, () => 'needs')
      await persist($)
      await $.prompt.submit({
        text: 'mygoal: the operator rejected the proposed check. Propose a different finish check that proves the goal in the plain meaning of the operator\'s words.',
      })
    }
    const stop = async () => {
      await update($, status, () => 'stopped')
      await update($, note, () => 'stopped by the operator')
      await persist($)
    }
    const resume = async () => {
      await update($, status, () => 'working')
      await update($, note, () => '')
      await persist($)
      await $.prompt.submit({ text: 'mygoal: the operator resumed the goal. Work on it now.' })
    }

    const label: Record<Status, string> = {
      none: 'no goal: type /mygoal <your goal>',
      needs: 'waiting for Claude to propose the finish check',
      approval: 'waiting for you to approve the finish check',
      working: `working, ${n} check${n === 1 ? '' : 's'} run, one for each new commit`,
      done: 'done: checked and judged',
      stopped: 'stopped',
    }

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" columnGap={2}>
          <Text bold color="cyan">mygoal</Text>
          <Text color={s === 'done' ? 'green' : s === 'stopped' ? 'yellow' : undefined}>{label[s]}</Text>
        </Box>
        {goal !== '' && <Text wrap="wrap">{goal}</Text>}
        {team.length > 0 && (
          <Box flexDirection="column">
            <Text bold>Agents</Text>
            {team.map(([name, a]) => (
              <Box key={name} flexDirection="column">
                <Text color={t - a.heardAt >= CHECK_IN_MS ? 'yellow' : undefined}>{`${name}: heard from ${ago(a.heardAt)}`}</Text>
                {a.said !== '' && <Text wrap="wrap">{`  doing: ${a.said}`}</Text>}
                {a.asked !== '' && <Text dimColor wrap="wrap">{`  asked: ${a.asked}`}</Text>}
              </Box>
            ))}
          </Box>
        )}
        {s === 'approval' && prop && (
          <Box flexDirection="column">
            <Text bold>Proposed finish check</Text>
            <Code source={prop.argv.join(' ')} />
            <Text dimColor wrap="wrap">{prop.why}</Text>
            <Box flexDirection="row" columnGap={3}>
              <Button key="approve" label="approve" hotkey="a" plain onPress={approve} />
              <Button key="reject" label="reject" hotkey="x" plain onPress={reject} />
            </Box>
          </Box>
        )}
        {check && (
          <Box flexDirection="column">
            <Text bold>Finish check</Text>
            <Code source={check.argv.join(' ')} />
          </Box>
        )}
        {run && (
          <Box flexDirection="column">
            <Text color={run.exitCode === 0 ? 'green' : 'red'}>{`check ${run.check} on ${run.commit.slice(0, 8)}: exited ${run.exitCode}`}</Text>
            <Code source={run.tail.split('\n').slice(-12).join('\n') || '(no output)'} />
            {run.verdict ? (
              <Box flexDirection="column">
                <Text color={run.verdict.met ? 'green' : 'red'}>{`judge: ${run.verdict.met ? 'met' : 'not met'}`}</Text>
                {run.verdict.missing !== '' && <Text wrap="wrap">{`missing: ${run.verdict.missing}`}</Text>}
                {run.verdict.outsideGoal !== '' && <Text wrap="wrap" color="yellow">{`outside the goal: ${run.verdict.outsideGoal}`}</Text>}
              </Box>
            ) : (
              <Text dimColor>judge: no answer</Text>
            )}
          </Box>
        )}
        {why !== '' && <Text dimColor>{why}</Text>}
        <Box flexDirection="row" columnGap={3}>
          {isOpen(s) && <Button key="stop" label="stop" hotkey="s" plain onPress={stop} />}
          {s === 'stopped' && check && <Button key="resume" label="resume" hotkey="r" plain onPress={resume} />}
        </Box>
      </Box>
    )
  })
}
