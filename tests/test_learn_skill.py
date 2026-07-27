import unittest
from pathlib import Path

import yaml


REPO = Path(__file__).resolve().parents[1]
LEARN_SKILL = REPO / "skills" / "learn" / "SKILL.md"


class LearnSkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = LEARN_SKILL.read_text(encoding="utf-8")

    def test_is_agent_neutral(self):
        _, frontmatter, _ = self.text.split("---", 2)
        self.assertEqual("learn", yaml.safe_load(frontmatter)["name"])
        self.assertIn("repository-authoritative", self.text)
        self.assertNotIn("~/.claude", self.text.lower())
        self.assertNotIn("~/.codex", self.text.lower())
        self.assertNotIn("wait for approval", self.text.lower())

    def test_reviews_every_repository_for_thirty_days(self):
        self.assertIn("30 days", self.text)
        self.assertIn("every Git repository", self.text)
        self.assertIn("git log", self.text)
        self.assertIn("git show", self.text)

    def test_routes_and_verifies_learnings(self):
        self.assertIn("reusable", self.text)
        self.assertIn("repository-specific", self.text)
        self.assertIn("existing skill", self.text)
        self.assertIn("validate", self.text)
        self.assertIn("commit", self.text)


if __name__ == "__main__":
    unittest.main()
