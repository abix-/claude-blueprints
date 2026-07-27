"""Tests for shared Claude and Codex blueprint installation."""

import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "sync_check",
    ROOT / "sync-check.py",
)
SYNC_CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SYNC_CHECK)


class SyncCheckTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.repo = self.root / "repo"
        self.home = self.root / "home"
        self.write("skills/shared/SKILL.md", "shared skill\n")
        self.write("skills/shared/references/notes.md", "shared notes\n")
        self.write("claude/skills/claude-only/SKILL.md", "claude skill\n")
        self.write("claude/CLAUDE.md", "claude rules\n")
        self.write("claude/settings.json", '{"hooks": {}}\n')
        self.write("codex/skills/codex-only/SKILL.md", "codex skill\n")
        self.write("codex/AGENTS.md", "codex rules\n")
        self.write("codex/hooks.json", '{"hooks": {}}\n')

    def tearDown(self):
        self.temp_dir.cleanup()

    def write(self, relative_path, content):
        path = self.repo / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_codex_install_combines_shared_and_codex_files(self):
        copied = SYNC_CHECK.install(self.repo, self.home, "codex")

        self.assertEqual(5, copied)
        self.assertEqual(
            "shared skill\n",
            (self.home / ".agents/skills/shared/SKILL.md").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(
            "shared notes\n",
            (self.home / ".agents/skills/shared/references/notes.md").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(
            "codex skill\n",
            (self.home / ".agents/skills/codex-only/SKILL.md").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(
            "codex rules\n",
            (self.home / ".codex/AGENTS.md").read_text(encoding="utf-8"),
        )
        self.assertEqual(
            '{"hooks": {}}\n',
            (self.home / ".codex/hooks.json").read_text(encoding="utf-8"),
        )
        self.assertFalse((self.home / ".claude").exists())

    def test_claude_install_combines_shared_and_claude_files(self):
        copied = SYNC_CHECK.install(self.repo, self.home, "claude")

        self.assertEqual(5, copied)
        self.assertTrue(
            (self.home / ".claude/skills/shared/SKILL.md").is_file()
        )
        self.assertTrue(
            (self.home / ".claude/skills/shared/references/notes.md").is_file()
        )
        self.assertTrue(
            (self.home / ".claude/skills/claude-only/SKILL.md").is_file()
        )
        self.assertTrue((self.home / ".claude/CLAUDE.md").is_file())
        self.assertTrue((self.home / ".claude/settings.json").is_file())
        self.assertFalse((self.home / ".codex").exists())

    def test_check_reports_changed_and_missing_files(self):
        SYNC_CHECK.install(self.repo, self.home, "codex")
        (self.home / ".codex/AGENTS.md").write_text(
            "changed rules\n",
            encoding="utf-8",
        )
        (self.home / ".agents/skills/shared/SKILL.md").unlink()

        rows = SYNC_CHECK.check(self.repo, self.home, "codex")
        statuses = {
            row.destination.relative_to(self.home).as_posix(): row.status
            for row in rows
        }

        self.assertEqual(
            "CHANGED",
            statuses[".codex/AGENTS.md"],
        )
        self.assertEqual(
            "MISSING",
            statuses[".agents/skills/shared/SKILL.md"],
        )
        self.assertEqual(
            "OK",
            statuses[".codex/hooks.json"],
        )

    def test_runtime_skill_cannot_replace_shared_skill(self):
        self.write("codex/skills/shared/SKILL.md", "replacement\n")

        with self.assertRaisesRegex(
            ValueError,
            "runtime skill conflicts with shared skill: shared",
        ):
            SYNC_CHECK.build_manifest(self.repo, self.home, "codex")

    def test_runtime_without_private_skills_is_valid(self):
        repo = self.root / "minimal-repo"
        shared = repo / "skills/shared/SKILL.md"
        shared.parent.mkdir(parents=True)
        shared.write_text("shared\n", encoding="utf-8")
        rules = repo / "codex/AGENTS.md"
        rules.parent.mkdir(parents=True)
        rules.write_text("rules\n", encoding="utf-8")

        manifest = SYNC_CHECK.build_manifest(repo, self.home, "codex")

        self.assertEqual(2, len(manifest))
        self.assertIn(self.home / ".agents/skills/shared/SKILL.md", manifest)
        self.assertIn(self.home / ".codex/AGENTS.md", manifest)

    def test_repository_has_distinct_claude_and_codex_runtime_files(self):
        claude_rules = ROOT / "claude/CLAUDE.md"
        codex_rules = ROOT / "codex/AGENTS.md"

        self.assertTrue(claude_rules.is_file())
        self.assertTrue((ROOT / "claude/settings.json").is_file())
        self.assertTrue(codex_rules.is_file())

        text = codex_rules.read_text(encoding="utf-8")
        self.assertIn("NEVER use subagents", text)
        self.assertIn("ALWAYS use PowerShell for shell commands on Windows", text)
        self.assertNotIn("ALWAYS use Bash for shell commands", text)
        self.assertIn("~/.agents/skills", text)
        self.assertIn("NEVER destroy uncommitted work", text)
        self.assertIn("NEVER `git stash`", text)
        self.assertIn("Volume is not progress", text)
        self.assertNotIn("## Failure log", text)
        self.assertNotIn("running record; the operator adds", text)
        self.assertNotIn("Violation 2026-", text)

    def test_windows_installer_installs_codex_runtime(self):
        with tempfile.TemporaryDirectory() as home:
            result = subprocess.run(
                [
                    "pwsh",
                    "-NoProfile",
                    "-File",
                    str(ROOT / "install.ps1"),
                    "-Runtime",
                    "codex",
                    "-HomePath",
                    home,
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(0, result.returncode, result.stderr)
            installed_home = Path(home)
            self.assertTrue((installed_home / ".codex/AGENTS.md").is_file())
            self.assertTrue(
                (installed_home / ".agents/skills/code/SKILL.md").is_file()
            )

    def test_runtime_specific_skills_are_not_shared(self):
        claude_only = {
            "claude-code-deep-dive",
            "claude-config",
            "fix-auth",
            "load",
            "why",
        }
        codex_only = {
            "codex-config",
            "codex-deep-dive",
            "source-command-load",
        }

        for name in claude_only:
            self.assertFalse((ROOT / "skills" / name / "SKILL.md").exists())
            self.assertTrue((ROOT / "claude/skills" / name / "SKILL.md").is_file())

        for name in codex_only:
            self.assertFalse((ROOT / "skills" / name / "SKILL.md").exists())
            self.assertTrue((ROOT / "codex/skills" / name / "SKILL.md").is_file())

    def test_codex_load_skill_uses_shared_windows_installer(self):
        text = (
            ROOT / "codex/skills/source-command-load/SKILL.md"
        ).read_text(encoding="utf-8")

        self.assertIn("C:\\code\\claude-blueprints", text)
        self.assertIn("install.ps1", text)
        self.assertIn('-Runtime "codex"', text)
        self.assertNotIn("Codex-blueprints", text)
        self.assertNotIn("```bash", text)
        self.assertNotIn("sanitizer", text)
        self.assertNotIn("Remove files", text)

    def test_claude_load_skill_uses_shared_windows_installer(self):
        text = (
            ROOT / "claude/skills/load/SKILL.md"
        ).read_text(encoding="utf-8")

        self.assertIn("C:/code/claude-blueprints", text)
        self.assertIn("install.ps1", text)
        self.assertIn('-Runtime "claude"', text)
        self.assertNotIn("sanitizer", text)
        self.assertNotIn("Remove files", text)

    def test_claude_session_hook_reads_installed_skill_directories(self):
        with tempfile.TemporaryDirectory() as home:
            SYNC_CHECK.install(ROOT, home, "claude")
            environment = os.environ.copy()
            environment["USERPROFILE"] = home
            result = subprocess.run(
                [
                    "pwsh",
                    "-NoProfile",
                    "-File",
                    str(
                        ROOT
                        / "claude/hooks/Hook-SessionStart-Skills.ps1"
                    ),
                ],
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(0, result.returncode, result.stderr)
            self.assertIn("# Try Harder", result.stdout)
            self.assertIn("# Code", result.stdout)
            self.assertIn("# Claude Config", result.stdout)


if __name__ == "__main__":
    unittest.main()
