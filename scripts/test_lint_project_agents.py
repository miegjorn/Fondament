"""Tests for the project-agent composition CI lint.

Run with: python -m unittest scripts/test_lint_project_agents.py
(or: python scripts/test_lint_project_agents.py)

These tests are the F-2 acceptance criteria. The invalid-file cases were written
*before* the lint logic existed (TDD): each constructs a deliberately broken
project-composition YAML and asserts the lint flags it with the expected rule.
"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from lint_project_agents import lint_dir  # noqa: E402


VALID = """\
id: fondament/projects/alpha-agent
kind: project-composition
name: "alpha-agent"
description: "Project agent for Alpha"
model: claude-sonnet-4-6
parts:
  - role: "development assistant"
    source: inline
    content: |
      You are the Alpha project agent.
  - role: context
    source: farga
    project: "alpha"
"""


def _projects_root(tmp):
    """Create the definitions/fondament/projects/ layout under tmp and return it."""
    root = os.path.join(tmp, "definitions", "fondament", "projects")
    os.makedirs(root, exist_ok=True)
    return root


def _write(root, name, content):
    with open(os.path.join(root, name), "w", encoding="utf-8") as fh:
        fh.write(content)


def _rules(findings, level=None):
    return {f.rule for f in findings if level is None or f.level == level}


class ProjectAgentLintTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.root = _projects_root(self.tmp)
        self.defs_root = os.path.join(self.tmp, "definitions")

    def _lint(self):
        return lint_dir(self.root, self.defs_root)

    # ── happy path ────────────────────────────────────────────────────────────
    def test_valid_composition_has_no_errors(self):
        _write(self.root, "alpha-agent.yaml", VALID)
        findings = self._lint()
        errors = [f for f in findings if f.level == "error"]
        self.assertEqual(errors, [], f"valid file produced errors: {errors}")

    # ── invalid cases (written first, TDD) ──────────────────────────────────────
    def test_wrong_kind_errors(self):
        _write(self.root, "alpha-agent.yaml", VALID.replace(
            "kind: project-composition", "kind: project-agent"))
        self.assertIn("kind-project-composition", _rules(self._lint(), "error"))

    def test_missing_id_errors(self):
        bad = "\n".join(l for l in VALID.splitlines()
                        if not l.startswith("id:")) + "\n"
        _write(self.root, "alpha-agent.yaml", bad)
        self.assertIn("id-present", _rules(self._lint(), "error"))

    def test_id_path_mismatch_errors(self):
        _write(self.root, "alpha-agent.yaml", VALID.replace(
            "id: fondament/projects/alpha-agent",
            "id: fondament/projects/wrong-name"))
        self.assertIn("id-path-convention", _rules(self._lint(), "error"))

    def test_missing_name_errors(self):
        bad = "\n".join(l for l in VALID.splitlines()
                        if not l.startswith("name:")) + "\n"
        _write(self.root, "alpha-agent.yaml", bad)
        self.assertIn("name-present", _rules(self._lint(), "error"))

    def test_missing_description_errors(self):
        bad = "\n".join(l for l in VALID.splitlines()
                        if not l.startswith("description:")) + "\n"
        _write(self.root, "alpha-agent.yaml", bad)
        self.assertIn("description-present", _rules(self._lint(), "error"))

    def test_missing_parts_errors(self):
        bad = """\
id: fondament/projects/alpha-agent
kind: project-composition
name: "alpha-agent"
description: "Project agent for Alpha"
model: claude-sonnet-4-6
"""
        _write(self.root, "alpha-agent.yaml", bad)
        self.assertIn("parts-present", _rules(self._lint(), "error"))

    def test_farga_part_without_project_errors(self):
        bad = """\
id: fondament/projects/alpha-agent
kind: project-composition
name: "alpha-agent"
description: "Project agent for Alpha"
parts:
  - role: context
    source: farga
"""
        _write(self.root, "alpha-agent.yaml", bad)
        self.assertIn("farga-part-project", _rules(self._lint(), "error"))

    def test_model_is_optional(self):
        bad = "\n".join(l for l in VALID.splitlines()
                        if not l.startswith("model:")) + "\n"
        _write(self.root, "alpha-agent.yaml", bad)
        errors = [f for f in self._lint() if f.level == "error"]
        self.assertEqual(errors, [], f"absent model must not error: {errors}")

    def test_deconstructive_field_warns_not_errors(self):
        _write(self.root, "alpha-agent.yaml", VALID + "deconstructive: true\n")
        findings = self._lint()
        self.assertIn("deconstructive-field", _rules(findings, "warn"))
        self.assertNotIn("deconstructive-field", _rules(findings, "error"))

    def test_invalid_yaml_errors(self):
        _write(self.root, "broken.yaml", "id: x\n  : : bad\n :\n")
        self.assertIn("yaml-parse", _rules(self._lint(), "error"))


if __name__ == "__main__":
    unittest.main()
