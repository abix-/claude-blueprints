import { expect, mock, test } from 'claude-code/testing'

type On = Parameters<Parameters<typeof test>[1]>[1]

const GOAL = 'every test in crates/foo passes and nothing else changes'
const CHECK = { argv: ['cargo', 'test', '-p', 'foo'], why: 'exit 0 means every foo test passed', repo: 'C:\\code\\foo' }

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
    checkSays: '',
    head: 'aaaaaaaa1111',
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
    if (e.argv[0] === 'git' && e.argv.includes('rev-parse')) return ok(w.head)
    if (e.argv[0] === 'git' && e.argv.includes('diff')) return ok('')
    if (e.argv[0] === 'git' && e.argv.includes('status')) return ok('')
    const queue = w.checkExit === 0 ? 'running a::one\n  passed\nrunning a::two\n  passed' : 'running 1 test\nrunning a::one\n  passed\nrunning a::two\n  FAILED\n[build] queue FAILED (exit 1)'
    return ok(w.checkSays || queue, w.checkExit)
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

test('the approved check runs once per commit, says nothing on the same commit, and ends when it passes and the judge agrees', async ($, on) => {
  const w = world(on)
  await start($, w)
  expect(w.submitted.at(-1)).toContain('Propose its check')

  // Claude proposes the check; nothing runs until the operator approves.
  await $.tool.call({ tool: 'mcp__mygoal__propose_check', input: CHECK } as never)
  const ui = await $.ui.mount({ plugin: 'mygoal', surface: 'terminal', component: 'Pane', requestId: 'mygoal', props: {} })
  expect(JSON.stringify(await ui.drawn())).toContain('waiting for you to approve the finish check')
  await ui.press({ key: 'approve' })
  expect(w.submitted.at(-1)).toContain('check approved')

  // The first turn after approval runs the check on the commit there.
  const ranCheck = () => w.ran.filter(a => a[0] === 'cargo').length
  await $.turn.complete(turnDone('Starting.'))
  expect(ranCheck()).toBe(1)
  expect(w.submitted.at(-1)).toContain('mygoal: not done')
  expect(w.submitted.at(-1)).toContain('exit 1 on aaaaaaaa')
  expect(w.submitted.at(-1)).toContain('1 passed; failed: a::two')
  expect(w.submitted.at(-1)).toContain('missing two foo tests fail')
  // Short: the raw output is summed up, never pasted.
  expect(w.submitted.at(-1)).not.toContain('running a::one')

  // The judge is Opus 5.5 and never sees what Claude said, only the goal
  // and the evidence.
  expect(w.judgedBy.at(-1)).toBe('claude-opus-5-5')
  expect(w.judged.at(-1)).toContain(GOAL)
  expect(w.judged.at(-1)).toContain('FAILED')
  expect(w.judged.at(-1)).not.toContain('Starting')

  // Turns on the same commit run nothing and say nothing.
  const said = w.submitted.length
  await $.turn.complete(turnDone('Waiting on the writer.'))
  await $.turn.complete(turnDone('Read the logs.'))
  expect(ranCheck()).toBe(1)
  expect(w.submitted.length).toBe(said)

  // A turn ending with a question to the operator is sent back once; asked
  // again right after, it goes to the operator.
  await $.turn.complete(turnDone('Two ways to go. Which do you want?'))
  expect(w.submitted.length).toBe(said + 1)
  expect(w.submitted.at(-1)).toContain("don't ask the operator")
  await $.turn.complete(turnDone('It reverses your ruling. Say go?'))
  expect(w.submitted.length).toBe(said + 1)

  // Claiming the goal done on that commit is sent back, once.
  await $.turn.complete(turnDone('The goal is done.'))
  expect(ranCheck()).toBe(1)
  expect(w.submitted.length).toBe(said + 2)
  expect(w.submitted.at(-1)).toContain('nothing committed since the last check')
  await $.turn.complete(turnDone('The goal is done.'))
  expect(w.submitted.length).toBe(said + 2)

  // A new commit: the check runs once, and the message says what changed.
  w.head = 'bbbbbbbb2222'
  w.checkSays = 'running a::one\n  FAILED\nrunning a::two\n  passed'
  await $.turn.complete(turnDone('Fixed one.'))
  expect(ranCheck()).toBe(2)
  expect(w.submitted.at(-1)).toContain('since aaaaaaaa: fixed a::two; broke a::one')

  // The check passes but the judge finds work outside the goal: not done.
  w.head = 'cccccccc3333'
  w.checkSays = ''
  w.checkExit = 0
  w.judgeSays = { met: true, missing: '', outside_goal: 'renamed bar.rs, which the goal did not ask for' }
  await $.turn.complete(turnDone('Fixed.'))
  expect(w.submitted.at(-1)).toContain('outside renamed bar.rs')

  // Check passes and the judge agrees with nothing outside: done, and no more turns.
  w.head = 'dddddddd4444'
  w.judgeSays = { met: true, missing: '', outside_goal: '' }
  const before = w.submitted.length
  await $.turn.complete(turnDone('Undid the rename.'))
  expect(w.submitted.length).toBe(before)
  expect(JSON.stringify(await ui.drawn())).toContain('done: checked and judged')
})

test('a check that cannot reach the game is not a result: told once, run again on a later turn', async ($, on) => {
  const w = world(on)
  await start($, w)
  await $.tool.call({ tool: 'mcp__mygoal__propose_check', input: CHECK } as never)
  const ui = await $.ui.mount({ plugin: 'mygoal', surface: 'terminal', component: 'Pane', requestId: 'mygoal', props: {} })
  await ui.press({ key: 'approve' })
  const ranCheck = () => w.ran.filter(a => a[0] === 'cargo').length

  w.checkSays = 'no topside game answering on http://127.0.0.1:15703/topside'
  await $.turn.complete(turnDone('Starting.'))
  expect(w.submitted.at(-1)).toContain("can't reach what it tests")
  expect(w.judged.length).toBe(0)
  const said = w.submitted.length
  await $.turn.complete(turnDone('Asked the writer to launch it.'))
  expect(ranCheck()).toBe(2)
  expect(w.submitted.length).toBe(said)

  // The game is back: the same commit is checked for real.
  w.checkSays = ''
  await $.turn.complete(turnDone('It is up.'))
  expect(ranCheck()).toBe(3)
  expect(w.judged.length).toBe(1)
  expect(w.submitted.at(-1)).toContain('exit 1 on aaaaaaaa')
})

test('while a goal is open, Claude cannot touch the goal store or the mod', async ($, on) => {
  const w = world(on)
  on('tool.call', async () => ({ result: 'ran' }) as never)
  await start($, w)
  const r = await $.tool.call({ tool: 'Edit', file_path: 'C:\\code\\claude-blueprints\\plugins\\mygoal\\hooks\\register.tsx', old_string: 'a', new_string: 'b' } as never)
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
  expect(w.submitted.at(-1)).toContain('no word from perf')
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
  expect(drawn).toContain('response: holding built (idle since 22:17)')
  expect(drawn).toContain('perf: heard from 22 min ago')
  expect(drawn).toContain('instruction: do 9p')
  // Idle since minute 19: checked on 2 minutes later, not 10.
  expect(w.submitted.at(-1)).toContain('idle: writer')
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
  expect(mine?.text).toContain("You don't decide done")
  // The rules a skill used to hold: how to propose the check, and the Never list.
  expect(mine?.text).toContain('hardest to pass falsely')
  expect(mine?.text).toContain('Never: a check narrower than the goal')

  // Working, with other sessions on the goal: Claude orchestrates.
  await $.tool.call({ tool: 'mcp__mygoal__propose_check', input: CHECK } as never)
  const ui = await $.ui.mount({ plugin: 'mygoal', surface: 'terminal', component: 'Pane', requestId: 'mygoal', props: {} })
  await ui.press({ key: 'approve' })
  const working = (await $.prompt.compose({
    model: 'claude-opus-5-5',
    promptModel: 'claude-opus-5-5',
    surfaces: ['terminal'],
    tools: [],
    outputStyle: null,
    traits: [],
  })) as unknown as { sections: { id: string; text: string }[] }
  expect(working.sections.find(s => s.id === 'mygoal:goal')?.text).toContain('you orchestrate')
})
