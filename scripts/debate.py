#!/usr/bin/env python3
"""debate - structured debate loop for multiple AI agents."""

import argparse
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

DEBATE_DIR = Path.home() / ".debate"
ACTIVE_DIR = DEBATE_DIR / "active"
ARCHIVE_DIR = DEBATE_DIR / "archive"

STATE_FILE = ACTIVE_DIR / "state.json"
MESSAGES_FILE = ACTIVE_DIR / "messages.jsonl"
GOAL_FILE = ACTIVE_DIR / "goal.md"
LOCK_FILE = ACTIVE_DIR / ".lock"

PHASES = ["GOAL", "PROPOSE", "REVIEW", "CONSENSUS", "IMPLEMENT", "VERIFY", "DONE"]


def now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def lock(f):
    if sys.platform == "win32":
        import msvcrt
        msvcrt.locking(f.fileno(), msvcrt.LK_LOCK, 1)
    else:
        import fcntl
        fcntl.flock(f, fcntl.LOCK_EX)


def unlock(f):
    if sys.platform == "win32":
        import msvcrt
        msvcrt.locking(f.fileno(), msvcrt.LK_UNLCK, 1)
    else:
        import fcntl
        fcntl.flock(f, fcntl.LOCK_UN)


def with_lock(fn):
    """Run fn while holding the debate lock file."""
    ACTIVE_DIR.mkdir(parents=True, exist_ok=True)
    lf = open(LOCK_FILE, "w")
    try:
        lock(lf)
        return fn()
    finally:
        unlock(lf)
        lf.close()


def load_state():
    if not STATE_FILE.exists():
        return None
    return json.loads(STATE_FILE.read_text())


def save_state(state):
    state["updated"] = now_iso()
    STATE_FILE.write_text(json.dumps(state, indent=2) + "\n")


def append_message(msg):
    msg["ts"] = now_iso()
    with open(MESSAGES_FILE, "a") as f:
        f.write(json.dumps(msg) + "\n")


def read_messages(last=None):
    if not MESSAGES_FILE.exists():
        return []
    lines = MESSAGES_FILE.read_text().strip().splitlines()
    msgs = [json.loads(line) for line in lines if line.strip()]
    if last is not None:
        msgs = msgs[-last:]
    return msgs


def require_state():
    state = load_state()
    if state is None:
        print("no active debate. run: debate new \"goal\"")
        sys.exit(1)
    return state


def get_session_id():
    sid = os.environ.get("CLAUDE_CODE_SESSION_ID")
    if not sid:
        print("CLAUDE_CODE_SESSION_ID not set. run this from inside a Claude Code session.")
        sys.exit(1)
    return sid


def get_agent(args, state):
    """Resolve agent identity from CLAUDE_CODE_SESSION_ID."""
    sid = get_session_id()
    for k, v in state["participants"].items():
        if v.get("session_id") == sid:
            return k
    print("this session has not joined the debate. run: debate join")
    sys.exit(1)


def swap_roles(state):
    old_proposer = state["proposer"]
    old_reviewer = state["reviewer"]
    state["proposer"] = old_reviewer
    state["reviewer"] = old_proposer
    state["round"] += 1


def resolve_body(msg):
    """Return the full body text, reading from file if the message uses one."""
    body_file = msg.get("body_file")
    if body_file:
        path = Path(body_file)
        if path.exists():
            return path.read_text(encoding="utf-8").strip()
        return f"(file missing: {body_file})"
    return msg.get("body", "")


def get_body(args):
    """Get proposal/review body from --file or from message args."""
    if hasattr(args, "file") and args.file:
        path = Path(args.file)
        if not path.exists():
            print(f"file not found: {args.file}")
            sys.exit(1)
        return path.read_text(encoding="utf-8").strip()
    if not args.message:
        print("provide a message or use --file to read from a file")
        sys.exit(1)
    return " ".join(args.message)


def save_body_file(body, label):
    """Save a long body to a file under the debate directory. Returns the path."""
    BODIES_DIR = ACTIVE_DIR / "bodies"
    BODIES_DIR.mkdir(parents=True, exist_ok=True)
    ts = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    filename = f"{label}-{ts}.md"
    path = BODIES_DIR / filename
    path.write_text(body + "\n", encoding="utf-8")
    return str(path)


def build_message_fields(body, label):
    """Return dict fields for the message. Uses a file for long bodies."""
    if len(body) > 500:
        body_file = save_body_file(body, label)
        return {"body": body[:200] + "...", "body_file": body_file}
    return {"body": body}


def format_message(msg):
    ts = msg.get("ts", "")
    if ts:
        ts = ts[11:19]
    sender = msg.get("from", "?")
    phase = msg.get("phase", "")
    rnd = msg.get("round", "")
    verdict = msg.get("verdict", "")
    body = resolve_body(msg)

    header = f"[{ts}] {sender}"
    if phase:
        header += f" ({phase}"
        if rnd:
            header += f" r{rnd}"
        if verdict:
            header += f" {verdict}"
        header += ")"
    print(f"{header}: {body}")


#. Commands.

def cmd_new(args):
    def do():
        if STATE_FILE.exists():
            print("a debate is already active. run: debate done")
            sys.exit(1)

        ACTIVE_DIR.mkdir(parents=True, exist_ok=True)
        ts = now_iso()
        debate_id = "debate-" + datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")

        state = {
            "id": debate_id,
            "phase": "GOAL",
            "round": 0,
            "turn": None,
            "proposer": None,
            "reviewer": None,
            "participants": {
                "human": {"name": "human", "type": "human"}
            },
            "created": ts,
            "updated": ts,
        }
        save_state(state)

        goal = " ".join(args.goal)
        GOAL_FILE.write_text(goal + "\n")
        append_message({"from": "human", "phase": "GOAL", "body": goal})
        print(f"debate started: {debate_id}")
        print(f"goal: {goal}")
        print()
        print("=== terminal 1 (your claude) ===")
        print(f"  claude")
        print(f"  then say: /debate")
        print()
        print("=== terminal 2 (other claude) ===")
        print(f"  claude")
        print(f"  then say: /debate")
        print()

    with_lock(do)


def cmd_join(args):
    def do():
        state = require_state()
        session_id = get_session_id()
        agent_type = args.type or "claude-code"

        for k, v in state["participants"].items():
            if v.get("session_id") == session_id:
                print(f"already joined as {k}")
                return

        agents = [k for k, v in state["participants"].items() if v["type"] != "human"]
        name = args.name
        if not name:
            name = f"agent-{chr(ord('a') + len(agents))}"

        if name in state["participants"]:
            print(f"{name} already taken")
            sys.exit(1)

        entry = {"name": name, "type": agent_type, "session_id": session_id}
        state["participants"][name] = entry
        append_message({"from": "system", "phase": state["phase"],
                        "body": f"{name} joined as {agent_type}"})

        agents = [k for k, v in state["participants"].items() if v["type"] != "human"]
        if len(agents) == 2 and state["phase"] == "GOAL":
            state["phase"] = "PROPOSE"
            state["round"] = 1
            state["proposer"] = agents[0]
            state["reviewer"] = agents[1]
            state["turn"] = agents[0]
            append_message({"from": "system", "phase": "PROPOSE",
                            "body": f"two agents joined. {agents[0]} proposes first. round 1."})

        save_state(state)
        print(f"joined as {name} ({agent_type})")

    with_lock(do)


def cmd_status(args):
    state = require_state()
    print(f"debate:   {state['id']}")
    print(f"phase:    {state['phase']}")
    print(f"round:    {state['round']}")
    print(f"turn:     {state['turn'] or '(none)'}")
    print(f"proposer: {state['proposer'] or '(none)'}")
    print(f"reviewer: {state['reviewer'] or '(none)'}")
    print(f"agents:   {', '.join(k for k in state['participants'] if state['participants'][k]['type'] != 'human')}")

    goal = GOAL_FILE.read_text().strip() if GOAL_FILE.exists() else "(none)"
    print(f"goal:     {goal}")


def cmd_read(args):
    msgs = read_messages(last=args.last)
    if not msgs:
        print("no messages yet")
        return
    for msg in msgs:
        format_message(msg)


def cmd_propose(args):
    def do():
        state = require_state()
        agent = get_agent(args, state)
        body = get_body(args)

        if state["phase"] != "PROPOSE":
            print(f"cannot propose in phase {state['phase']}")
            sys.exit(1)
        if state["turn"] != agent:
            print(f"not your turn. current turn: {state['turn']}")
            sys.exit(1)
        agents = [k for k in state["participants"] if state["participants"][k]["type"] != "human"]
        other = [a for a in agents if a != agent]
        state["proposer"] = agent
        if other:
            state["reviewer"] = other[0]

        msg = {"from": agent, "phase": "PROPOSE", "round": state["round"]}
        msg.update(build_message_fields(body, f"propose-{agent}-r{state['round']}"))
        append_message(msg)
        state["phase"] = "REVIEW"
        state["turn"] = state["reviewer"]
        save_state(state)
        print(f"proposal submitted. {state['turn']}'s turn to review.")

    with_lock(do)


def cmd_review(args):
    def do():
        state = require_state()
        agent = get_agent(args, state)
        verdict = args.verdict
        body = get_body(args)

        if state["phase"] != "REVIEW":
            print(f"cannot review in phase {state['phase']}")
            sys.exit(1)
        if state["turn"] != agent:
            print(f"not your turn. current turn: {state['turn']}")
            sys.exit(1)
        agents = [k for k in state["participants"] if state["participants"][k]["type"] != "human"]
        other = [a for a in agents if a != agent]
        state["reviewer"] = agent
        if other:
            state["proposer"] = other[0]

        msg = {"from": agent, "phase": "REVIEW", "round": state["round"],
               "verdict": verdict}
        msg.update(build_message_fields(body, f"review-{agent}-r{state['round']}"))
        append_message(msg)

        if verdict == "agree":
            state["phase"] = "CONSENSUS"
            state["turn"] = None
            append_message({"from": "system", "phase": "CONSENSUS",
                            "body": "both agents agree. waiting for human to advance to IMPLEMENT or send back."})
        else:
            swap_roles(state)
            state["phase"] = "PROPOSE"
            state["turn"] = state["proposer"]
            append_message({"from": "system", "phase": "PROPOSE",
                            "body": f"disagreement. roles swapped. {state['proposer']} proposes in round {state['round']}."})

        save_state(state)

    with_lock(do)


def cmd_say(args):
    def do():
        state = require_state()
        body = get_body(args)
        msg = {"from": "human", "phase": state["phase"]}
        msg.update(build_message_fields(body, "say-human"))
        append_message(msg)
        print("message sent")

    with_lock(do)


def cmd_force(args):
    def do():
        state = require_state()
        phase = args.phase.upper()
        if phase not in PHASES:
            print(f"unknown phase: {phase}. valid: {', '.join(PHASES)}")
            sys.exit(1)

        agents = [k for k in state["participants"] if state["participants"][k]["type"] != "human"]
        forced_agent = args.agent

        if forced_agent and forced_agent not in agents:
            print(f"unknown agent: {forced_agent}. joined: {', '.join(agents)}")
            sys.exit(1)

        old = state["phase"]
        state["phase"] = phase

        if phase == "PROPOSE":
            if forced_agent:
                state["proposer"] = forced_agent
                state["reviewer"] = [a for a in agents if a != forced_agent][0] if len(agents) >= 2 else None
            elif state["proposer"] is None and len(agents) >= 2:
                state["proposer"] = agents[0]
                state["reviewer"] = agents[1]
            state["turn"] = state["proposer"]
            state["round"] += 1

        elif phase == "IMPLEMENT":
            state["turn"] = forced_agent or state["proposer"]

        elif phase == "VERIFY":
            state["turn"] = forced_agent or state["reviewer"]

        elif phase == "REVIEW":
            if forced_agent:
                state["reviewer"] = forced_agent
                state["proposer"] = [a for a in agents if a != forced_agent][0] if len(agents) >= 2 else None
            state["turn"] = state["reviewer"]

        elif phase == "DONE":
            state["turn"] = None

        save_state(state)
        append_message({"from": "human", "phase": phase,
                        "body": f"human forced phase: {old} -> {phase}" + (f", turn: {forced_agent}" if forced_agent else "")})
        print(f"phase changed: {old} -> {phase}" + (f", turn: {forced_agent}" if forced_agent else f", turn: {state['turn']}"))

    with_lock(do)


def cmd_implement(args):
    def do():
        state = require_state()
        agent = get_agent(args, state)
        body = get_body(args)

        if state["phase"] not in ("CONSENSUS", "IMPLEMENT"):
            print(f"cannot implement in phase {state['phase']}")
            sys.exit(1)

        state["phase"] = "IMPLEMENT"
        state["turn"] = agent
        save_state(state)
        msg = {"from": agent, "phase": "IMPLEMENT", "round": state["round"]}
        msg.update(build_message_fields(body, f"implement-{agent}-r{state['round']}"))
        append_message(msg)
        print("implementation recorded.")

    with_lock(do)


def cmd_verify(args):
    def do():
        state = require_state()
        agent = get_agent(args, state)
        verdict = args.verdict
        body = get_body(args)

        if state["phase"] != "IMPLEMENT":
            print(f"cannot verify in phase {state['phase']}")
            sys.exit(1)

        msg = {"from": agent, "phase": "VERIFY", "round": state["round"],
               "verdict": verdict}
        msg.update(build_message_fields(body, f"verify-{agent}-r{state['round']}"))
        append_message(msg)

        if verdict == "accept":
            state["phase"] = "DONE"
            state["turn"] = None
            append_message({"from": "system", "phase": "DONE",
                            "body": "implementation accepted. debate complete."})
        else:
            state["phase"] = "PROPOSE"
            swap_roles(state)
            state["turn"] = state["proposer"]
            append_message({"from": "system", "phase": "PROPOSE",
                            "body": f"implementation rejected. back to debate. {state['proposer']} proposes in round {state['round']}."})

        save_state(state)

    with_lock(do)


def cmd_done(args):
    import shutil
    state = require_state()
    ARCHIVE_DIR.mkdir(parents=True, exist_ok=True)
    dest = ARCHIVE_DIR / state["id"]
    shutil.copytree(ACTIVE_DIR, dest)
    shutil.rmtree(ACTIVE_DIR)
    print(f"debate archived: {dest}")


def cmd_watch(args):
    if not MESSAGES_FILE.exists():
        print("no active debate")
        sys.exit(1)

    seen = 0
    print("watching debate (ctrl-c to stop)\n")

    goal = GOAL_FILE.read_text().strip() if GOAL_FILE.exists() else ""
    if goal:
        print(f"GOAL: {goal}\n")

    while True:
        msgs = read_messages()
        if len(msgs) > seen:
            for msg in msgs[seen:]:
                format_message(msg)
            seen = len(msgs)
        time.sleep(1)


def main():
    parser = argparse.ArgumentParser(prog="debate", description="structured agent debate")
    sub = parser.add_subparsers(dest="command")

    p = sub.add_parser("new", help="start a new debate")
    p.add_argument("goal", nargs="+", help="the goal")

    p = sub.add_parser("join", help="join the debate")
    p.add_argument("name", nargs="?", default=None, help="agent name (auto-assigned if omitted)")
    p.add_argument("--type", default="claude-code", help="agent type (claude-code, codex)")

    sub.add_parser("status", help="show debate status")

    p = sub.add_parser("read", help="read messages")
    p.add_argument("--last", type=int, default=None, help="show last N messages")

    p = sub.add_parser("propose", help="submit a proposal")
    p.add_argument("--file", default=None, help="read body from this file instead of args")
    p.add_argument("message", nargs="*", default=[])

    p = sub.add_parser("review", help="review a proposal")
    p.add_argument("verdict", choices=["agree", "disagree", "revise"])
    p.add_argument("--file", default=None, help="read body from this file instead of args")
    p.add_argument("message", nargs="*", default=[])

    p = sub.add_parser("say", help="human sends a message")
    p.add_argument("--file", default=None, help="read body from this file instead of args")
    p.add_argument("message", nargs="*", default=[])

    p = sub.add_parser("force", help="human forces a phase change")
    p.add_argument("phase", help="target phase")
    p.add_argument("agent", nargs="?", default=None, help="assign turn to this agent (e.g. agent-a)")

    p = sub.add_parser("implement", help="record implementation")
    p.add_argument("--file", default=None, help="read body from this file instead of args")
    p.add_argument("message", nargs="*", default=[])

    p = sub.add_parser("verify", help="verify implementation")
    p.add_argument("verdict", choices=["accept", "reject"])
    p.add_argument("--file", default=None, help="read body from this file instead of args")
    p.add_argument("message", nargs="*", default=[])

    sub.add_parser("done", help="archive the debate")
    sub.add_parser("watch", help="watch the debate live")

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        sys.exit(1)

    cmds = {
        "new": cmd_new, "join": cmd_join, "status": cmd_status,
        "read": cmd_read, "propose": cmd_propose, "review": cmd_review,
        "say": cmd_say, "force": cmd_force, "implement": cmd_implement,
        "verify": cmd_verify, "done": cmd_done, "watch": cmd_watch,
    }
    cmds[args.command](args)


if __name__ == "__main__":
    main()
