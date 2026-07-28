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
        self.assertIn("self-learning loop", lower)
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
        self.assertIn("resume the in-flight batch automatically", text)

    def test_factoriobot_loads_every_authoritative_design_doc_before_work(self):
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
        self.assertIn("read every file completely before selecting or changing work", text)
        self.assertIn("not fully present in the current context", text)
        self.assertIn("reread it completely from disk", text)
        self.assertIn("after compaction", text)

    def test_factoriobot_keeps_every_action_on_the_operators_goal_chain(self):
        text = skill("factoriobot").lower()
        self.assertIn("goal-alignment gate", text)
        self.assertIn("permanent gameplay goal", text)
        self.assertIn("current gameplay acceptance", text)
        self.assertIn("selected authority batch", text)
        self.assertIn("governing design statements", text)
        self.assertIn("measured gaps", text)
        self.assertIn("advances a recorded gameplay acceptance measure", text)
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


if __name__ == "__main__":
    unittest.main()
