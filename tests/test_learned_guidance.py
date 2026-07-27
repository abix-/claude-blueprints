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


if __name__ == "__main__":
    unittest.main()
