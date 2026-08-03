import unittest
import json
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[1]
ALLOWED_KEYS = {"name", "description", "allowed-tools", "license", "metadata"}


def skill_files():
    yield from sorted((ROOT / "skills").glob("*/SKILL.md"))
    yield from sorted((ROOT / "codex/skills").glob("*/SKILL.md"))


class SkillCompatibilityTests(unittest.TestCase):
    def test_known_user_skills_are_repository_authoritative(self):
        for name in {"authority-audit", "cleanup", "horsey"}:
            with self.subTest(name=name):
                self.assertTrue((ROOT / "skills" / name / "SKILL.md").is_file())
        self.assertTrue(
            (
                ROOT
                / "codex/skills/source-command-ctop/SKILL.md"
            ).is_file()
        )

    def test_shared_and_codex_skills_have_valid_frontmatter(self):
        for path in skill_files():
            with self.subTest(path=path.relative_to(ROOT)):
                text = path.read_text(encoding="utf-8")
                self.assertTrue(text.startswith("---\n"))
                _, frontmatter, _ = text.split("---", 2)
                data = yaml.safe_load(frontmatter)
                self.assertIsInstance(data, dict)
                self.assertEqual(path.parent.name, data.get("name"))
                self.assertIsInstance(data.get("description"), str)
                self.assertTrue(data["description"].strip())
                self.assertFalse(set(data) - ALLOWED_KEYS)

    def test_claude_skills_and_settings_parse(self):
        for path in sorted((ROOT / "claude/skills").glob("*/SKILL.md")):
            with self.subTest(path=path.relative_to(ROOT)):
                text = path.read_text(encoding="utf-8")
                self.assertTrue(text.startswith("---\n"))
                _, frontmatter, _ = text.split("---", 2)
                data = yaml.safe_load(frontmatter)
                self.assertIsInstance(data, dict)
                self.assertIsInstance(data.get("description"), str)
                self.assertTrue(data["description"].strip())

        settings = json.loads(
            (ROOT / "claude/settings.json").read_text(encoding="utf-8")
        )
        self.assertIsInstance(settings, dict)
        self.assertIn("permissions", settings)
        self.assertIn("hooks", settings)

    def test_installed_text_files_are_ascii(self):
        roots = [ROOT / "skills", ROOT / "claude", ROOT / "codex"]
        paths = []
        for root in roots:
            paths.extend(path for path in root.rglob("*") if path.is_file())
        paths.extend([ROOT / "sync.ps1", ROOT / "sync.md"])

        for path in sorted(paths):
            with self.subTest(path=path.relative_to(ROOT)):
                text = path.read_text(encoding="utf-8")
                non_ascii = sorted({character for character in text if ord(character) > 127})
                self.assertFalse(non_ascii, repr(non_ascii))

    def test_shared_skills_do_not_use_claude_skill_paths(self):
        for path in sorted((ROOT / "skills").glob("*/SKILL.md")):
            with self.subTest(path=path.relative_to(ROOT)):
                text = path.read_text(encoding="utf-8")
                self.assertNotIn("~/.claude/skills", text)
                self.assertNotIn("global CLAUDE.md", text)
                self.assertNotIn("drives the CLI through Bash", text)

    def test_obey_supports_both_runtime_instruction_files(self):
        text = (ROOT / "skills/obey/SKILL.md").read_text(encoding="utf-8")
        self.assertIn("~/.claude/CLAUDE.md", text)
        self.assertIn("~/.codex/AGENTS.md", text)
        self.assertIn("current runtime", text)


if __name__ == "__main__":
    unittest.main()
