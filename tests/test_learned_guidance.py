import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]


def skill(name):
    return (REPO / "skills" / name / "SKILL.md").read_text(encoding="utf-8")


class LearnedGuidanceTests(unittest.TestCase):
    def test_code_covers_behavior_evidence_and_safe_migrations(self):
        text = skill("code")
        self.assertIn("Measured behavior", text)
        self.assertIn("Source text is valid only for proving an absence", text)
        self.assertIn("locked safety behavior", text)
        self.assertIn("same initialization path", text)

    def test_csharp_covers_verified_harmony_resolution_and_writeback(self):
        text = skill("csharp")
        self.assertIn("exact full-name", text)
        self.assertIn("short-name fallback", text)
        self.assertIn("embedded Harmony version", text)
        self.assertIn("boxed copy", text)

    def test_eufy_covers_later_verified_detection_and_control_flow(self):
        text = skill("eufy")
        self.assertIn("one movement", text)
        self.assertIn("ground_truth.jsonl", text)
        self.assertIn("distinct result variants", text)
        self.assertIn("ollama", text)

    def test_factoriobot_covers_current_desired_state_design(self):
        text = skill("factoriobot")
        self.assertIn("factory audit", text)
        self.assertIn("missing, nonfunctional, wrong, extra, and satisfied", text)
        self.assertIn("existing work pool", text)
        self.assertIn("authored parent requirements", text)
        self.assertIn("same completion condition", text)
        self.assertIn("coal bootstrap", text)
        self.assertIn("copper base", text)

    def test_factoriobot_persists_every_unattended_status_report(self):
        text = skill("factoriobot")
        lower = text.lower()
        self.assertIn("docs/status-reports.md", text)
        self.assertIn("before publishing it in chat", lower)
        self.assertIn("same report", lower)
        self.assertIn("append-only", lower)

    def test_factoriobot_closes_the_self_learning_loop(self):
        text = skill("factoriobot")
        lower = text.lower()
        self.assertIn("self-learning requirement", lower)
        self.assertIn("applicable shared skill", lower)
        self.assertIn("owning project docs", lower)
        self.assertIn("same verified batch", lower)
        self.assertIn("validate, commit, and push both repositories", lower)

    def test_factoriobot_uses_factorio_prototype_filter_arrays(self):
        text = skill("factoriobot")
        self.assertIn("prototype filter methods require an array of typed filter tables", text)
        self.assertIn('get_entity_filtered{{filter="type",type="mining-drill"}}', text)

    def test_factoriobot_reports_only_gameplay_milestones(self):
        text = skill("factoriobot").lower()
        self.assertIn("milestones reached contains only actual gameplay progress", text)
        self.assertIn("code, tests, builds, commits, skills, and docs are not milestones", text)
        self.assertIn("verified progress and checks", text)
        self.assertIn("reach the factorio endgame with 100% automation", text)
        self.assertIn("measurable progress toward that goal", text)
        self.assertIn("endgame automation progress:", text)
        self.assertIn("completed and automated", text)
        self.assertIn("prototype-derived route to the space age endgame", text)
        self.assertIn("player help does not count as automated", text)

    def test_factoriobot_recovers_current_instructions_after_compaction(self):
        text = skill("factoriobot").lower()
        self.assertIn("after any context compaction", text)
        self.assertIn("carried instruction text is recovery evidence only", text)
        self.assertIn("current filesystem `agents.md`", text)
        self.assertIn("active matching skills", text)
        self.assertIn("before taking another action", text)
        self.assertIn("read only enough current project state", text)
        self.assertIn("resume it automatically", text)
        self.assertIn(
            "do not reload every design document merely because context compacted",
            text,
        )

    def test_factoriobot_loads_every_authoritative_design_doc_only_for_selection(self):
        text = skill("factoriobot").lower()
        for path in (
            "docs/design.md",
            "docs/authority.md",
            "docs/construction.md",
            "docs/brain.md",
            "docs/body.md",
            "docs/framework.md",
            "docs/efficiency.md",
            "docs/todo.md",
            "docs/design-resolution-plan.md",
        ):
            self.assertIn(path, text)
        self.assertIn("before selecting or changing to a new authority batch", text)
        self.assertIn(
            "recovering a clearly recorded in-flight batch uses only its owning "
            "documents and current evidence",
            " ".join(text.split()),
        )
        self.assertIn(
            "bare invocation resumes a clearly recorded in-flight batch without "
            "repeating this gate",
            " ".join(text.split()),
        )

    def test_factoriobot_keeps_every_action_on_the_operators_goal_chain(self):
        text = skill("factoriobot").lower()
        self.assertIn("goal-alignment gate", text)
        self.assertIn("permanent gameplay goal", text)
        self.assertIn("current gameplay acceptance", text)
        self.assertIn("selected authority batch", text)
        self.assertIn("governing design statements", text)
        self.assertIn("measured gaps", text)
        self.assertIn("advances a recorded gameplay acceptance measure", text)

    def test_factoriobot_stops_support_work_that_no_longer_advances_the_goal(self):
        text = skill("factoriobot").lower()
        self.assertIn("goal-value stop", text)
        self.assertIn("minimum valid support artifact", text)
        self.assertIn("operator's gameplay goal", text)
        self.assertIn("required gate", text)
        self.assertIn("return to gameplay work", text)
        self.assertIn("removes a documented authority bypass required for that acceptance", text)
        self.assertIn("if neither statement is true, do not perform the action", text)
        self.assertIn("supporting evidence, never progress by themselves", text)
        self.assertIn("review the complete run", text)
        self.assertIn("complete the whole design-aligned authority batch", text)
        self.assertIn("before another acceptance run", text)

    def test_factoriobot_requires_parameterized_framework_analysis_before_selection(self):
        text = skill("factoriobot").lower()
        self.assertIn("parameterized framework analysis", text)
        self.assertIn("shared authority failure", text)
        self.assertIn("canonical framework boundary", text)
        self.assertIn("parameters that vary", text)
        self.assertIn("every producer and consumer", text)
        self.assertIn("authority score before and target", text)
        self.assertIn("gameplay measure before and target", text)
        self.assertIn("reject it as a hotfix", text)
        self.assertIn("keep analyzing", text)

    def test_factoriobot_turns_support_work_into_one_gameplay_progress_gate(self):
        text = skill("factoriobot").lower()
        self.assertIn("repository development workflow", text)
        self.assertIn("one combined verification gate", text)
        self.assertIn("the next objective is gameplay movement", text)
        self.assertIn("do not present support actions as separate progress steps", text)
        self.assertIn("tests, documentation, status, commits, pushes, and review", text)
        self.assertIn("one acceptance attempt", text)
        self.assertIn("if gameplay does not move", text)
        self.assertIn("hooks enforce only mechanical facts", text)
        self.assertIn("hooks do not own judgment", text)

    def test_factoriobot_keeps_hooks_mechanical_and_operator_control_absolute(self):
        text = " ".join(skill("factoriobot").lower().split())
        self.assertIn("the latest operator message always wins", text)
        self.assertIn("ordinary operator request suspends automatic continuation", text)
        self.assertIn("stop hooks", text)
        self.assertIn("destructive actions", text)
        self.assertIn("exact tested and pushed commit", text)
        self.assertIn("canonical review, recorded root cause, and committed failing proof", text)
        self.assertIn("repeating the same completion", text)
        self.assertIn("resolve every path from current files with `rg --files`", text)
        self.assertIn("resolve every command, api, function, test location, and hook permission", text)
        self.assertIn("never guess repository details", text)
        self.assertNotIn("the current phase is authoritative", text)
        self.assertNotIn("tracked project hook blocks production edits", text)

    def test_factoriobot_rejects_attempt_regressions_with_generic_red_proofs(self):
        text = " ".join(skill("factoriobot").lower().split())
        self.assertIn("after every attempt", text)
        self.assertIn("complete attempt history", text)
        self.assertIn("previously achieved gameplay capability", text)
        self.assertIn("regression is a failed implementation", text)
        self.assertIn("generic red proof", text)
        self.assertIn("shared authority concept", text)
        self.assertIn("not one attempt, building, resource, or symptom", text)
        self.assertIn("do not start another acceptance attempt", text)
        self.assertIn("meet or exceed the historical capability", text)

    def test_factoriobot_uses_one_data_driven_development_loop(self):
        text = " ".join(skill("factoriobot").lower().split())
        self.assertIn("repository development workflow", text)
        self.assertIn("run the current relevant test set", text)
        self.assertIn("diagnose the shared root cause", text)
        self.assertIn("generic, dry, parameterized red proof", text)
        self.assertIn("fix the shared implementation", text)
        self.assertIn("rerun the complete affected test set", text)
        self.assertIn("compare the new results with the recorded baseline", text)
        self.assertIn("tests are permanent progression data", text)
        self.assertIn("never weaken, delete, or narrow a passing proof", text)
        self.assertIn("repeat from the next highest-impact proven gap", text)
        self.assertIn("the red test queue is the work queue", text)
        self.assertIn("resolve every known red proof before selecting new work", text)
        self.assertIn("do not start a new feature, authority batch, or acceptance attempt", text)
        self.assertIn("group red proofs by their shared root cause", text)
        self.assertIn("make the entire selected group green", text)
        self.assertIn(
            "all other sections are gates, requirements, or reference details "
            "inside this workflow, not alternate workflows",
            text,
        )
        self.assertEqual(text.count("repository development workflow"), 1)
        self.assertNotIn("gameplay-progress execution loop", text)
        self.assertNotIn("repository development loop", text)
        self.assertNotIn("development cycle", text)
        self.assertNotIn("self-learning loop", text)


if __name__ == "__main__":
    unittest.main()
