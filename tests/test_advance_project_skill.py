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
            "status record",
            "changelog",
        ):
            self.assertIn(term, self.text)

    def test_workflow_requires_red_first_and_product_acceptance(self):
        self.assertIn("failing behavioral proofs before production code", self.text)
        self.assertIn("exact tested and pushed build", self.text)
        self.assertIn("Do not hotfix and rerun.", self.text)

    def test_operator_outcome_is_the_only_progress_measure(self):
        self.assertIn("Preserve the operator's exact outcome", self.text)
        self.assertIn("Only measured product-state movement is progress.", self.text)
        self.assertIn("Support artifacts are not milestones.", self.text)

    def test_recovery_resumes_current_work_from_disk(self):
        self.assertIn("After compaction or interruption", self.text)
        self.assertIn("read them again from disk", self.text)
        self.assertIn("resume the in-flight batch", self.text)

    def test_batch_records_root_cause_scores_and_acceptance_before_code(self):
        self.assertIn("Record the root cause and coherent batch before production code", self.text)
        self.assertIn("current and target score", self.text)
        self.assertIn("measured product acceptance", self.text)

    def test_live_evidence_and_project_vocabulary_are_authoritative(self):
        self.assertIn("current live state", self.text)
        self.assertIn("historical evidence cannot prove current state", self.text)
        self.assertIn("established project and domain terminology", self.text)

    def test_hooks_only_enforce_mechanical_gates(self):
        self.assertIn("Hooks may enforce only mechanical gates", self.text)
        self.assertIn("never block operator conversation", self.text)

    def test_learning_returns_to_the_product_batch(self):
        self.assertIn("update the owning skill", self.text)
        self.assertIn("resume the product batch", self.text)


if __name__ == "__main__":
    unittest.main()
