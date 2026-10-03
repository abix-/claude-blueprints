import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { Check, Run, Status, Verdict } from '../types'

// /mygoal: keeps Claude on the operator's goal until it is truly done.
//
// Claude does not decide "done". Two things outside Claude do:
//   the finish check: a command Claude proposes and the operator approves
//     with a button only the operator can press; this mod runs it itself
//     after every turn and reads the exit code,
//   the judge: a separate call to Opus 5.5 that sees only the goal in the
//     operator's words, the check's real output and the code changes since
//     approval, never Claude's explanation. It says what is missing and
//     what was done that the goal did not ask for.
// Until the check exits 0 and the judge says met with nothing outside the
// goal, the next turn starts by itself with what is missing.
//
// The stock /goal is left alone; this is a separate command to compare.

const PANE = 'mygoal'
const TOOL = 'propose_check'
const JUDGE_MODEL = 'claude-opus-5-5'
const DEFAULT_LIMIT = 30
const TAIL_CHARS = 4000
const CHANGES_CHARS = 40000

const goalText = atom({ plugin: 'mygoal', key: 'text' } as const, '')
const status = atom({ plugin: 'mygoal', key: 'status' } as const, 'none')
const proposed = atom({ plugin: 'mygoal', key: 'proposed' } as const, null)
const approved = atom({ plugin: 'mygoal', key: 'approved' } as const, null)
const turns = atom({ plugin: 'mygoal', key: 'turns' } as const, 0)
const limit = atom({ plugin: 'mygoal', key: 'limit' } as const, DEFAULT_LIMIT)
const last = atom({ plugin: 'mygoal', key: 'last' } as const, null)
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

// Is the goal one that keeps Claude working?
const isOpen = (s: Status) => s === 'needs' || s === 'approval' || s === 'working'

// Save everything a later session needs to carry on.
async function persist($: EngineInterface) {
  await $.store.set('goal', {
    text: await read($, goalText),
    status: await read($, status),
    proposed: await read($, proposed),
    approved: await read($, approved),
    turns: await read($, turns),
    limit: await read($, limit),
    last: await read($, last),
    note: await read($, note),
  })
}

async function load($: EngineInterface) {
  const saved = (await $.store.get('goal')) as
    | { text: string; status: Status; proposed: Check | null; approved: Check | null; turns: number; limit: number; last: Run | null; note: string }
    | undefined
  if (!saved) return
  await update($, goalText, () => saved.text)
  await update($, status, () => saved.status)
  await update($, proposed, () => saved.proposed)
  await update($, approved, () => saved.approved)
  await update($, turns, () => saved.turns)
  await update($, limit, () => saved.limit)
  await update($, last, () => saved.last)
  await update($, note, () => saved.note)
}

// git diff HEAD split per file, plus the untracked files: what the working
// tree looks like now, so the judge sees only what changed after approval.
async function treeNow($: EngineInterface): Promise<Record<string, string>> {
  const out: Record<string, string> = {}
  const diff = await $.process.run(['git', 'diff', 'HEAD'])
  if (diff.exitCode !== 0) return out
  for (const part of diff.stdout.split(/^(?=diff --git )/m)) {
    const name = /^diff --git a\/(.+?) b\//.exec(part)?.[1]
    if (name) out[name] = part
  }
  const st = await $.process.run(['git', 'status', '--porcelain'])
  for (const line of st.stdout.split('\n')) {
    if (line.startsWith('?? ')) out[line.slice(3).trim()] = '(new file, not tracked yet)'
  }
  return out
}

async function changesSinceApproval($: EngineInterface): Promise<string> {
  const before = ((await $.store.get('baseline')) ?? {}) as Record<string, string>
  const now = await treeNow($)
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

// After a working turn: run the check, ask the judge, then either finish
// or start the next turn with what is missing.
async function evaluate($: EngineInterface) {
  const check = await read($, approved)
  if (!check) return
  const turn = (await read($, turns)) + 1
  const max = await read($, limit)
  if (turn > max) {
    await update($, status, () => 'stopped')
    await update($, note, () => `stopped: the limit of ${max} checked turns was reached`)
    await persist($)
    $.ui.toast(`mygoal: stopped at the ${max} turn limit`)
    return
  }
  await update($, turns, () => turn)

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
  const changes = await changesSinceApproval($)
  const goal = await read($, goalText)
  const verdict = await judge($, goal, check, exitCode, tail, changes)
  await update($, last, () => ({ turn, exitCode, tail, verdict }))

  if (exitCode === 0 && verdict?.met && verdict.outsideGoal.trim() === '') {
    await update($, status, () => 'done')
    await update($, note, () => `done on turn ${turn}: the check exited 0 and the judge agreed`)
    await persist($)
    $.ui.toast('mygoal: done, checked and judged')
    return
  }
  await persist($)
  await $.prompt.submit({
    text: [
      `mygoal: the goal is not done yet (checked turn ${turn} of ${max}).`,
      'The goal, in the operator\'s words:',
      goal,
      '',
      `The finish check \`${check.argv.join(' ')}\` exited ${exitCode}. Last part of its output:`,
      tail || '(no output)',
      '',
      verdict
        ? `The judge (a separate Opus 5.5 call that sees only the goal, this output and the code changes) says\nmissing: ${verdict.missing || '(nothing)'}\noutside the goal: ${verdict.outsideGoal || '(nothing)'}`
        : 'The judge did not answer this time.',
      '',
      'Keep working on the goal. Undo anything outside the goal. Only the check passing and the judge agreeing end this.',
    ].join('\n'),
  })
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
        'Propose the finish check for the operator\'s /mygoal goal: a command whose exit code 0 proves the goal is met in the plain meaning of the operator\'s words. The operator approves or rejects it; until approved, do no goal work.',
      inputSchema: {
        type: 'object',
        properties: {
          argv: { type: 'array', items: { type: 'string' }, description: 'The command and its arguments, run with no shell from the session directory' },
          why: { type: 'string', description: 'Why exit code 0 proves the whole goal' },
        },
        required: ['argv', 'why'],
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
      await update($, turns, () => 0)
      await update($, last, () => null)
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
    const input = (e as unknown as { input?: { argv?: unknown; why?: unknown } }).input ?? (e as unknown as { argv?: unknown; why?: unknown })
    const argv = Array.isArray(input.argv) ? input.argv.map(String) : []
    if (argv.length === 0) return { result: 'argv must be the command and its arguments, at least one item.' }
    await update($, proposed, () => ({ argv, why: String(input.why ?? '') }))
    await update($, status, () => 'approval')
    await persist($)
    void $.ui.open({ id: PANE, title: 'mygoal' })
    return {
      result:
        'Proposed. The operator approves or rejects it in the mygoal pane. Stop here and wait for their answer; do not start the goal work until the check is approved.',
    }
  })

  // While a goal is open, nothing Claude runs may touch where the goal is
  // kept (the plugin store) or this mod's code, which lives in the mygoal
  // skill's folder (the repo's claude/skills/mygoal, installed to
  // ~/.claude/skills/mygoal).
  on('tool.call', async ($, e, next) => {
    if (!isOpen(await read($, status)) || e.tool.startsWith('mcp__mygoal__')) return next(e)
    const asked = JSON.stringify(e)
    if (/plugins[\\/]+store/i.test(asked) || /skills[\\/]+mygoal/i.test(asked)) {
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
        ? 'Before any other work, propose the finish check with the propose_check tool: a command whose exit code 0 proves the goal is met in the plain meaning of the operator\'s words.'
        : s === 'approval'
          ? 'Your proposed finish check is waiting for the operator\'s approval. Do no goal work until it is approved.'
          : 'Work on this goal and nothing else. Do nothing the goal did not ask for. Read the docs first, then make the decisions yourself and keep the work moving; ask the operator only before the work starts, never wait on them mid-goal. The operator changes what they disagree with.'
    const text = [
      'The operator\'s goal (/mygoal), in their words:',
      await read($, goalText),
      '',
      'You do not decide when this goal is done. After every turn the approved finish check is run for you and a separate judge reads the goal from its output and your code changes alone. Until both agree, the work goes on.',
      step,
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
    const n = await read($, turns)
    const max = await read($, limit)
    const run = await read($, last)
    const why = await read($, note)
    const team = Object.entries(await read($, agents)).sort(([a], [b]) => a.localeCompare(b))
    const t = await read($, now)
    const ago = (at: number) => (t - at < 60000 ? 'just now' : `${Math.floor((t - at) / 60000)} min ago`)

    const approve = async () => {
      const p = await read($, proposed)
      if (!p) return
      await $.store.set('baseline', await treeNow($))
      await update($, approved, () => p)
      await update($, proposed, () => null)
      await update($, status, () => 'working')
      await update($, turns, () => 0)
      await update($, last, () => null)
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
      await update($, turns, () => 0)
      await update($, note, () => '')
      await persist($)
      await $.prompt.submit({ text: 'mygoal: the operator resumed the goal. Work on it now.' })
    }

    const label: Record<Status, string> = {
      none: 'no goal: type /mygoal <your goal>',
      needs: 'waiting for Claude to propose the finish check',
      approval: 'waiting for you to approve the finish check',
      working: `working, checked turn ${n} of ${max}`,
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
            <Text color={run.exitCode === 0 ? 'green' : 'red'}>{`turn ${run.turn}: the check exited ${run.exitCode}`}</Text>
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
