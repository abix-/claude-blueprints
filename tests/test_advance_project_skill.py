import unittest
from pathlib import Path

import yaml


REPO = Path(__file__).resolve().parents[1]
SKILL = REPO / "skills" / "advance-project" / "SKILL.md"


class AdvanceProjectSkillTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = SKILL.read_text(encoding="utf-8")
        _, frontmatter, _ = cls.text.split("---", 2)
        cls.metadata = yaml.safe_load(frontmatter)

    def test_trigger_covers_the_complete_project_workflow(self):
        self.assertEqual("advance-project", self.metadata["name"])
        description = self.metadata["description"].lower()
        for term in ("design", "authority", "todo", "issue", "changelog", "acceptance"):
            self.assertIn(term, description)

    def test_one_authority_owns_each_fact_or_decision(self):
        self.assertIn("One fact or decision has one authority.", self.text)
        self.assertIn("Observers provide facts", self.text)
        self.assertIn("executors apply selected work", self.text)

    def test_documents_have_distinct_lifecycle_roles(self):
        for term in (
            "owning design doc",
            "authority map",
            "todo or issue",
            "project state",
            "changelog",
        ):
            self.assertIn(term, self.text)

    def test_workflow_requires_red_first_and_product_acceptance(self):
        self.assertIn("failing behavioral proofs before production code", self.text)
        self.assertIn("Run one acceptance on the exact tested build.", self.text)
        self.assertIn("Do not hotfix and rerun.", self.text)


if __name__ == "__main__":
    unittest.main()
