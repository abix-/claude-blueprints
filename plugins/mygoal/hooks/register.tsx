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
  await $.prompt.submit({ text: `mygoal: no word from ${silent.join(', ')} in 10 min. Check each, give it its next step.` })
}

const JUDGE = [
  'You judge whether an operator\'s goal is met. You see only the goal, in the operator\'s own words,',
  'the finish check the operator approved with its real output, and the code changes made for it.',
  'You do not see the worker\'s explanation, and nothing it could say changes the evidence.',
  'Read the goal\'s words in their plain everyday meaning. Do not accept a narrower reading of any word.',
  'If any part of the goal is not shown done by the evidence, the goal is not met.',
  'Also list everything in the code changes that the goal did not ask for.',
  'Answer with JSON only, no other text:',
  '{"met": true or false, "missing": "what is not done yet, at most 30 words, or empty", "outside_goal": "changes the goal did not ask for, at most 30 words, or empty"}',
].join('\n')

// How to propose the finish check: in the tool's description, read at every
// call, and in the request while a check is needed.
const PROPOSING =
  'Propose the check hardest to pass falsely: it fails while any part of the goal is unmet, covers the whole goal, runs against the real thing, no shell. For a game: the queue run of the goal\'s tests, committed first. No command can prove it: say so in why.'

// What never happens while a goal is open, in every request.
const NEVER =
  'Never: a check narrower than the goal; a new check to dodge a failure; ignoring "not done yet"; touching this mod or its store; /goal alongside.'

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

// A check's output read as test results: each "running <test>" line paired
// with the passed or failed line after it, and the last line that is
// neither (where a run stopped).
function results(tail: string): { passed: string[]; failed: string[]; last: string } {
  const out = { passed: [] as string[], failed: [] as string[], last: '' }
  let test = ''
  for (const line of tail.split('\n').map(l => l.trim()).filter(l => l !== '')) {
    const running = /^running (\S+::\S+)$/.exec(line)?.[1]
    if (running) test = running
    else if (test !== '' && /^passed$/i.test(line)) out.passed.push(test)
    else if (test !== '' && /^failed$/i.test(line)) out.failed.push(test)
    else if (!/^running \d+ tests?$/.test(line)) out.last = line
  }
  return out
}

// The "not done yet" message, a few lines: passes counted, failures named,
// what changed since the check before, where it stopped, the judge.
function notDone(run: Run, before: Run | null, why: string): string {
  const now = results(run.tail)
  const was = before ? results(before.tail) : null
  const fixed = was ? was.failed.filter(t => now.passed.includes(t)) : []
  const broke = was ? was.passed.filter(t => now.failed.includes(t)) : []
  const tested = now.passed.length + now.failed.length > 0
  return [
    `mygoal: not done ${why}, exit ${run.exitCode} on ${run.commit.slice(0, 8)}.`,
    tested ? `${now.passed.length} passed; failed: ${now.failed.join(', ') || '-'}` : `output: ${run.tail.split('\n').slice(-3).join(' | ')}`,
    fixed.length + broke.length > 0 ? `since ${was && before ? before.commit.slice(0, 8) : ''}: fixed ${fixed.join(', ') || '-'}; broke ${broke.join(', ') || '-'}` : '',
    tested && now.last !== '' ? `last: ${now.last}` : '',
    run.verdict ? `judge: missing ${run.verdict.missing || '-'}; outside ${run.verdict.outsideGoal || '-'}` : 'judge: no answer',
  ]
    .filter(l => l !== '')
    .join('\n')
}

// After a working turn: Claude does not stop until the check passes. On a
// new commit in the check's repo the check runs once and its result starts
// the next turn; on a commit already checked, the next turn starts at once
// with the last result. Only the check passing with the judge agreeing, or
// the operator's stop, ends it.
async function evaluate($: EngineInterface) {
  const check = await read($, approved)
  if (!check) return
  const repo = repoOf(check)
  const head = await headOf($, repo)
  const goal = await read($, goalText)
  const before = await read($, last)

  if (head === (await read($, checked))) {
    await $.prompt.submit({ text: before ? notDone(before, null, '(no new commit)') : 'mygoal: not done. Keep working.' })
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

  // Not a test result: the commit stays unchecked, so the next turn runs the
  // check again.
  if (exitCode !== 0 && UNREACHABLE.test(tail)) {
    await update($, note, () => `check ${n} on ${head.slice(0, 8)} could not reach what it tests`)
    await persist($)
    await $.prompt.submit({ text: `mygoal: the check can't reach what it tests (${head.slice(0, 8)}); get it running.\n${tail.split('\n').slice(-3).join('\n')}` })
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
  await $.prompt.submit({ text: notDone(run, before, `(check ${n})`) })
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
        `Propose the /mygoal finish check: exit 0 proves the goal met. The operator approves it. ${PROPOSING}`,
      inputSchema: {
        type: 'object',
        properties: {
          argv: { type: 'array', items: { type: 'string' }, description: 'Command and arguments, no shell' },
          why: { type: 'string', description: 'Why exit 0 proves the whole goal' },
          repo: { type: 'string', description: 'Absolute path of the git repo; the check runs on each new commit there' },
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
          text: 'mygoal: new goal. Propose its check with propose_check first.',
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
        ? `First propose the check with propose_check. ${PROPOSING}`
        : s === 'approval'
          ? 'The check awaits the operator\'s approval; no goal work until then.'
          : 'Only this goal. You make every decision and get every step done: never ask the operator what to do, never leave an agent waiting on them. With other sessions on it, you orchestrate: they edit, build and test; you decide, direct, check and keep them working.'
    const text = [
      `Goal (/mygoal): ${await read($, goalText)}`,
      'You don\'t decide done: the check runs on each new commit and a judge reads its output. ' + step,
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
        await evaluate($)
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
      await $.prompt.submit({ text: 'mygoal: check approved. Work on the goal.' })
    }
    const reject = async () => {
      await update($, proposed, () => null)
      await update($, status, () => 'needs')
      await persist($)
      await $.prompt.submit({
        text: 'mygoal: check rejected. Propose another.',
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
      await $.prompt.submit({ text: 'mygoal: resumed. Work on the goal.' })
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
                {a.asked !== '' && <Text dimColor wrap="wrap">{`  instruction: ${a.asked}`}</Text>}
                {a.said !== '' && <Text wrap="wrap">{`  response: ${a.said}`}</Text>}
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
