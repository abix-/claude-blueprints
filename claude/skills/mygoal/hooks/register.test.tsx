import { expect, mock, test } from 'claude-code/testing'

type On = Parameters<Parameters<typeof test>[1]>[1]

const GOAL = 'every test in crates/foo passes and nothing else changes'
const CHECK = { argv: ['cargo', 'test', '-p', 'foo'], why: 'exit 0 means every foo test passed' }

// Stand in for the engine beneath the mod: the store, the pane, prompts it
// sends, the commands it runs, and the judge model. `world.checkExit` and
// `world.judgeSays` decide what the check and the judge answer.
function world(on: On) {
  const w = {
    submitted: [] as string[],
    judged: [] as string[],
    judgedBy: [] as string[],
    ran: [] as string[][],
    checkExit: 1,
    judgeSays: { met: false, missing: 'two foo tests fail', outside_goal: '' } as Record<string, unknown>,
  }
  mock.store(on)
  const clock = mock.clock(on)
  Object.assign(w, { clock })
  on('ui.open', async () => ({ value: { isPlaced: true } }))
  on('ui.toast', async () => ({ value: undefined }))
  on('command.register', async () => ({ value: undefined }))
  on('tool.register', async () => ({ value: { tool: 'mcp__mygoal__propose_check' } }))
  on('session.start', async (_$, e) => ({ cwd: e.cwd }))
  on('turn.complete', async (_$, e) => ({ text: e.answer }))
  on('prompt.submit', async (_$, e) => {
    w.submitted.push(e.text)
    return { text: e.text }
  })
  on('process.run', async (_$, e) => {
    w.ran.push([...e.argv])
    const ok = (stdout: string, exitCode = 0) => ({
      value: { exitCode, stdout, stderr: '', isStdoutTruncated: false, isStderrTruncated: false },
    })
    if (e.argv[0] === 'git' && e.argv[1] === 'diff') return ok('')
    if (e.argv[0] === 'git' && e.argv[1] === 'status') return ok('')
    return ok(w.checkExit === 0 ? 'test result: ok. 12 passed' : 'test result: FAILED. 10 passed; 2 failed', w.checkExit)
  })
  on('model.complete', async (_$, e) => {
    w.judged.push(`${e.system}\n---\n${e.prompt}`)
    w.judgedBy.push(e.model)
    return { value: { isAnswered: true, text: JSON.stringify(w.judgeSays), usage: { input_tokens: 1, output_tokens: 1 } } }
  })
  return w as typeof w & { clock: typeof clock }
}

async function start($: Parameters<Parameters<typeof test>[1]>[0], w: ReturnType<typeof world>) {
  await $.session.start({ cwd: '.', surface: 'terminal', isInteractive: true })
  // As the operator typing it at the prompt.
  await $.command.run({ command: 'mygoal', args: GOAL, origin: { kind: 'composer' } } as never)
  // The first prompt goes out just after the command returns.
  await w.clock.advance(0)
}

const turnDone = (answer: string) => ({ answer, durationMs: 1, isAborted: false, turnId: 't', reason: 'answer' as const })

test('a goal needs an approved check; until it passes and the judge agrees, the next turn starts by itself', async ($, on) => {
  const w = world(on)
  await start($, w)
  expect(w.submitted.at(-1)).toContain('propose its finish check')

  // Claude proposes the check; nothing runs until the operator approves.
  await $.tool.call({ tool: 'mcp__mygoal__propose_check', input: CHECK } as never)
  const ui = await $.ui.mount({ plugin: 'mygoal', surface: 'terminal', component: 'Pane', requestId: 'mygoal', props: {} })
  expect(JSON.stringify(await ui.drawn())).toContain('waiting for you to approve the finish check')
  await ui.press({ key: 'approve' })
  expect(w.submitted.at(-1)).toContain('approved the finish check `cargo test -p foo`')

  // Claude says it is done. The check fails: the mod sends the next turn.
  await $.turn.complete(turnDone('All done! Every test passes now.'))
  expect(w.ran).toContainEqual(CHECK.argv)
  expect(w.submitted.at(-1)).toContain('the goal is not done yet (checked turn 1 of 30)')
  expect(w.submitted.at(-1)).toContain('exited 1')
  expect(w.submitted.at(-1)).toContain('missing: two foo tests fail')

  // The judge is Opus 5.5 and never sees what Claude said, only the goal
  // and the evidence.
  expect(w.judgedBy.at(-1)).toBe('claude-opus-5-5')
  expect(w.judged.at(-1)).toContain(GOAL)
  expect(w.judged.at(-1)).toContain('2 failed')
  expect(w.judged.at(-1)).not.toContain('All done')

  // The check passes but the judge finds work outside the goal: not done.
  w.checkExit = 0
  w.judgeSays = { met: true, missing: '', outside_goal: 'renamed bar.rs, which the goal did not ask for' }
  await $.turn.complete(turnDone('Fixed.'))
  expect(w.submitted.at(-1)).toContain('outside the goal: renamed bar.rs')

  // Check passes and the judge agrees with nothing outside: done, and no more turns.
  w.judgeSays = { met: true, missing: '', outside_goal: '' }
  const before = w.submitted.length
  await $.turn.complete(turnDone('Undid the rename.'))
  expect(w.submitted.length).toBe(before)
  expect(JSON.stringify(await ui.drawn())).toContain('done: checked and judged')
})

test('while a goal is open, Claude cannot touch the goal store or the mod', async ($, on) => {
  const w = world(on)
  on('tool.call', async () => ({ result: 'ran' }) as never)
  await start($, w)
  const r = await $.tool.call({ tool: 'Edit', file_path: '~/.claude/skills/mygoal/hooks/register.tsx', old_string: 'a', new_string: 'b' } as never)
  expect(JSON.stringify(r)).toContain('not Claude')
  const s = await $.tool.call({ tool: 'Bash', command: 'cat ~/.claude/plugins/store/mygoal.json' } as never)
  expect(JSON.stringify(s)).toContain('not Claude')
})

test('while a goal works, an agent silent for 10 minutes gets a check-up turn', async ($, on) => {
  const w = world(on)
  on('session.send', async () => ({ isDelivered: true }) as never)
  on('session.receive', async (_$, e) => ({ text: e.text }) as never)
  await start($, w)
  await $.tool.call({ tool: 'mcp__mygoal__propose_check', input: CHECK } as never)
  const ui = await $.ui.mount({ plugin: 'mygoal', surface: 'terminal', component: 'Pane', requestId: 'mygoal', props: {} })
  await ui.press({ key: 'approve' })

  await $.session.send({ to: 'writer', text: 'build holding' } as never)
  await $.session.send({ to: 'perf', text: 'do 9p' } as never)
  await w.clock.advance(5 * 60 * 1000)
  // The writer answers; the perf session stays silent.
  await $.session.receive({ origin: { kind: 'peer' }, text: '<cross-session-message from-name="writer">holding built</cross-session-message>' } as never)
  const before = w.submitted.length
  await w.clock.advance(6 * 60 * 1000)
  expect(w.submitted.length).toBe(before + 1)
  expect(w.submitted.at(-1)).toContain('no word from perf in 10 minutes')
  expect(w.submitted.at(-1)).not.toContain('writer')

  // Checked on at minute 10: no second check-up before minute 20.
  await w.clock.advance(8 * 60 * 1000)
  expect(w.submitted.filter(t => t.includes('no word from perf')).length).toBe(1)

  // The pane shows each agent: when it was last heard from, what it last
  // said, and what it was last asked; an idle notice keeps what it said.
  await $.session.receive({ origin: { kind: 'unclassified' }, text: '[Cross-session idle notice] "writer", which you asked to be notified about, is idle now — it finished a turn at 22:17.' } as never)
  await w.clock.advance(3 * 60 * 1000)
  const drawn = JSON.stringify(await ui.drawn())
  expect(drawn).toContain('writer: heard from 3 min ago')
  expect(drawn).toContain('doing: holding built (idle since 22:17)')
  expect(drawn).toContain('perf: heard from 22 min ago')
  expect(drawn).toContain('asked: do 9p')
})

test('the goal is in every request while it is open', async ($, on) => {
  const w = world(on)
  on('prompt.compose', async () => ({ sections: [{ id: 'intro', text: 'base', scope: 'shared' }] }) as never)
  await start($, w)
  const r = (await $.prompt.compose({
    model: 'claude-opus-5-5',
    promptModel: 'claude-opus-5-5',
    surfaces: ['terminal'],
    tools: [],
    outputStyle: null,
    traits: [],
  })) as unknown as { sections: { id: string; text: string }[] }
  const mine = r.sections.find(s => s.id === 'mygoal:goal')
  expect(mine?.text).toContain(GOAL)
  expect(mine?.text).toContain('You do not decide when this goal is done')
})
