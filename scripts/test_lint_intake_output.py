"""Tests for the Intake Agent output CI lint (Fondament F-3).

Run with:
    python -m unittest scripts/test_lint_intake_output.py

These tests are the F-3 acceptance criteria.  Each invalid-file case was
written to specify the desired behaviour before the lint logic existed (TDD):
construct a deliberately broken file and assert the lint flags the right rule.
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from lint_intake_output import (  # noqa: E402
    lint_capabilities,
    lint_domain,
    lint_generated,
    lint_role,
)

# ---------------------------------------------------------------------------
# Canonical valid fixtures (slug = "nervi")
# ---------------------------------------------------------------------------

VALID_DOMAIN = """\
id: domain/nervi
kind: domain
repo: Nervi
default_facet: architect
context: |
  Nervi is the async subscription fabric of the Occitan stack.

  ## What Nervi is
  A NATS JetStream deployment plus an MCP server.

  ## Design constraints
  - NATS JetStream only.

  ## Dependencies
  - Farga: writes anomaly signals.
"""

VALID_ROLE_ARCHITECT = """\
id: fondament/nervi-architect
kind: role
default_model: claude-sonnet-4-6
context: |
  You are the architect agent for Nervi.

  You own the NATS topology and MCP API surface.
tools:
  always_on:
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
    - id: farga-write-signal
      kind: mcp
      server: farga
      tool: write_signal
    - id: farga-search-signals
      kind: mcp
      server: farga
      tool: search_signals
  jit: []
skills: []
"""

VALID_ROLE_DEVELOPER = """\
id: fondament/nervi-developer
kind: role
default_model: claude-sonnet-4-6
context: |
  You are a developer agent for Nervi.
tools:
  always_on:
    - id: bash
      kind: native
      tool: Bash
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
    - id: farga-write-signal
      kind: mcp
      server: farga
      tool: write_signal
  jit: []
skills: []
"""

VALID_CAPABILITIES = """\
project: nervi

exposes:
  - id: nervi-mcp
    kind: mcp-tool
    description: MCP server exposing nervi_publish and nervi_subscribe
    endpoint: http://nervi.occitan-system.svc.cluster.local:8080/mcp

consumes:
  - project: farga
    capability: write-signal
    required: false
"""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _setup_project(tmp: str, slug: str = "nervi") -> tuple[str, str]:
    """Create a project directory tree under tmp. Returns (project_dir, slug)."""
    project_dir = os.path.join(tmp, slug)
    os.makedirs(os.path.join(project_dir, "domains"), exist_ok=True)
    os.makedirs(os.path.join(project_dir, "roles"), exist_ok=True)
    return project_dir, slug


def _write(path: str, content: str) -> None:
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(content)


def _rules(findings, level=None):
    return {f.rule for f in findings if level is None or f.level == level}


def _write_valid_project(project_dir: str, slug: str = "nervi") -> None:
    """Populate a project directory with all four valid files."""
    _write(os.path.join(project_dir, "domains", f"{slug}.yaml"), VALID_DOMAIN)
    _write(os.path.join(project_dir, "roles", f"{slug}-architect.yaml"),
           VALID_ROLE_ARCHITECT)
    _write(os.path.join(project_dir, "roles", f"{slug}-developer.yaml"),
           VALID_ROLE_DEVELOPER)
    _write(os.path.join(project_dir, "capabilities.yaml"), VALID_CAPABILITIES)


# ---------------------------------------------------------------------------
# domain.yaml tests
# ---------------------------------------------------------------------------

class DomainLintTests(unittest.TestCase):

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.path = os.path.join(self.tmp, "nervi.yaml")

    # happy path
    def test_valid_domain_no_errors(self):
        _write(self.path, VALID_DOMAIN)
        findings = lint_domain(self.path, "nervi")
        errors = [f for f in findings if f.level == "error"]
        self.assertEqual(errors, [], f"valid domain produced errors: {errors}")

    # yaml-parse
    def test_invalid_yaml_errors(self):
        _write(self.path, "id: x\n  : bad\n :\n")
        self.assertIn("yaml-parse", _rules(lint_domain(self.path, "nervi"), "error"))

    def test_non_mapping_yaml_errors(self):
        _write(self.path, "- item1\n- item2\n")
        self.assertIn("yaml-parse", _rules(lint_domain(self.path, "nervi"), "error"))

    # domain-kind
    def test_wrong_kind_errors(self):
        _write(self.path, VALID_DOMAIN.replace("kind: domain", "kind: role"))
        self.assertIn("domain-kind", _rules(lint_domain(self.path, "nervi"), "error"))

    # domain-id-present / domain-id-format
    def test_missing_id_errors(self):
        bad = "\n".join(
            line for line in VALID_DOMAIN.splitlines()
            if not line.startswith("id:")
        ) + "\n"
        _write(self.path, bad)
        self.assertIn("domain-id-present", _rules(lint_domain(self.path, "nervi"), "error"))

    def test_wrong_id_format_errors(self):
        _write(self.path, VALID_DOMAIN.replace("id: domain/nervi", "id: fondament/nervi"))
        self.assertIn("domain-id-format", _rules(lint_domain(self.path, "nervi"), "error"))

    # domain-repo-present
    def test_missing_repo_errors(self):
        bad = "\n".join(
            line for line in VALID_DOMAIN.splitlines()
            if not line.startswith("repo:")
        ) + "\n"
        _write(self.path, bad)
        self.assertIn("domain-repo-present", _rules(lint_domain(self.path, "nervi"), "error"))

    # domain-context-present
    def test_missing_context_errors(self):
        # Replace the entire multi-line context block with nothing
        _write(self.path, "id: domain/nervi\nkind: domain\nrepo: Nervi\n")
        self.assertIn("domain-context-present", _rules(lint_domain(self.path, "nervi"), "error"))

    # optional fields must NOT trigger errors
    def test_default_facet_is_optional(self):
        bad = "\n".join(
            line for line in VALID_DOMAIN.splitlines()
            if not line.startswith("default_facet:")
        ) + "\n"
        _write(self.path, bad)
        errors = [f for f in lint_domain(self.path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"absent default_facet must not error: {errors}")


# ---------------------------------------------------------------------------
# roles/*.yaml tests
# ---------------------------------------------------------------------------

class RoleLintTests(unittest.TestCase):

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.arch_path = os.path.join(self.tmp, "nervi-architect.yaml")
        self.dev_path = os.path.join(self.tmp, "nervi-developer.yaml")

    def test_valid_architect_no_errors(self):
        _write(self.arch_path, VALID_ROLE_ARCHITECT)
        errors = [f for f in lint_role(self.arch_path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"valid architect produced errors: {errors}")

    def test_valid_developer_no_errors(self):
        _write(self.dev_path, VALID_ROLE_DEVELOPER)
        errors = [f for f in lint_role(self.dev_path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"valid developer produced errors: {errors}")

    # role-kind
    def test_wrong_kind_errors(self):
        _write(self.arch_path, VALID_ROLE_ARCHITECT.replace("kind: role", "kind: domain"))
        self.assertIn("role-kind", _rules(lint_role(self.arch_path, "nervi"), "error"))

    # role-id-present / role-id-format
    def test_missing_id_errors(self):
        bad = "\n".join(
            line for line in VALID_ROLE_ARCHITECT.splitlines()
            if not line.startswith("id:")
        ) + "\n"
        _write(self.arch_path, bad)
        self.assertIn("role-id-present", _rules(lint_role(self.arch_path, "nervi"), "error"))

    def test_wrong_id_format_errors(self):
        _write(self.arch_path,
               VALID_ROLE_ARCHITECT.replace(
                   "id: fondament/nervi-architect",
                   "id: domain/nervi-architect"))
        self.assertIn("role-id-format", _rules(lint_role(self.arch_path, "nervi"), "error"))

    # role-model-present
    def test_missing_model_errors(self):
        bad = "\n".join(
            line for line in VALID_ROLE_ARCHITECT.splitlines()
            if not line.startswith("default_model:")
        ) + "\n"
        _write(self.arch_path, bad)
        self.assertIn("role-model-present",
                      _rules(lint_role(self.arch_path, "nervi"), "error"))

    # role-context-present
    def test_missing_context_errors(self):
        _write(self.arch_path,
               "id: fondament/nervi-architect\nkind: role\n"
               "default_model: claude-sonnet-4-6\n"
               "tools:\n  always_on: []\n  jit: []\n")
        self.assertIn("role-context-present",
                      _rules(lint_role(self.arch_path, "nervi"), "error"))

    # role-tools-present
    def test_missing_tools_errors(self):
        bad = "\n".join(
            line for line in VALID_ROLE_ARCHITECT.splitlines()
            if not line.startswith("tools:")
        ) + "\n"
        _write(self.arch_path, bad)
        self.assertIn("role-tools-present",
                      _rules(lint_role(self.arch_path, "nervi"), "error"))

    def test_tools_not_mapping_errors(self):
        # Replace the tools block entirely with a scalar value
        lines = VALID_ROLE_ARCHITECT.splitlines()
        kept = []
        skip = False
        for line in lines:
            if line.startswith("tools:"):
                kept.append("tools: not-a-mapping")
                skip = True
                continue
            if skip and (line.startswith("skills:") or not line.startswith(" ")):
                skip = False
            if not skip:
                kept.append(line)
        _write(self.arch_path, "\n".join(kept) + "\n")
        self.assertIn("role-tools-present",
                      _rules(lint_role(self.arch_path, "nervi"), "error"))

    # role-tools-farga warning
    def test_missing_farga_tool_warns(self):
        # Remove farga-read-context and farga-write-signal
        stripped = (VALID_ROLE_ARCHITECT
                    .replace("    - id: farga-read-context\n"
                             "      kind: mcp\n"
                             "      server: farga\n"
                             "      tool: read_context\n", "")
                    .replace("    - id: farga-write-signal\n"
                             "      kind: mcp\n"
                             "      server: farga\n"
                             "      tool: write_signal\n", ""))
        _write(self.arch_path, stripped)
        findings = lint_role(self.arch_path, "nervi")
        self.assertIn("role-tools-farga", _rules(findings, "warn"))
        self.assertNotIn("role-tools-farga", _rules(findings, "error"))

    # skills is optional
    def test_skills_optional(self):
        bad = "\n".join(
            line for line in VALID_ROLE_ARCHITECT.splitlines()
            if not line.startswith("skills:")
        ) + "\n"
        _write(self.arch_path, bad)
        errors = [f for f in lint_role(self.arch_path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"absent skills must not error: {errors}")


# ---------------------------------------------------------------------------
# capabilities.yaml tests
# ---------------------------------------------------------------------------

class CapabilitiesLintTests(unittest.TestCase):

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.path = os.path.join(self.tmp, "capabilities.yaml")

    def test_valid_capabilities_no_errors(self):
        _write(self.path, VALID_CAPABILITIES)
        errors = [f for f in lint_capabilities(self.path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"valid capabilities produced errors: {errors}")

    # cap-project-present
    def test_missing_project_errors(self):
        bad = "\n".join(
            line for line in VALID_CAPABILITIES.splitlines()
            if not line.startswith("project:")
        ) + "\n"
        _write(self.path, bad)
        self.assertIn("cap-project-present",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    def test_wrong_project_slug_errors(self):
        _write(self.path, VALID_CAPABILITIES.replace("project: nervi", "project: wrong"))
        self.assertIn("cap-project-present",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    # cap-exposes-list
    def test_exposes_not_list_errors(self):
        # Replace exposes block with a scalar value (valid YAML, wrong type)
        _write(self.path,
               "project: nervi\nexposes: not-a-list\nconsumes: []\n")
        self.assertIn("cap-exposes-list",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    # cap-empty-exposes warning
    def test_empty_exposes_warns(self):
        _write(self.path,
               "project: nervi\nexposes: []\nconsumes: []\n")
        findings = lint_capabilities(self.path, "nervi")
        self.assertIn("cap-empty-exposes", _rules(findings, "warn"))
        self.assertNotIn("cap-empty-exposes", _rules(findings, "error"))

    # cap-expose-id, cap-expose-kind
    def test_expose_missing_id_errors(self):
        _write(self.path,
               "project: nervi\n"
               "exposes:\n  - kind: mcp-tool\n    description: foo\n"
               "consumes: []\n")
        self.assertIn("cap-expose-id",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    def test_expose_missing_kind_errors(self):
        _write(self.path,
               "project: nervi\n"
               "exposes:\n  - id: nervi-mcp\n    description: foo\n"
               "consumes: []\n")
        self.assertIn("cap-expose-kind",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    # cap-consume-project, cap-consume-capability
    def test_consume_missing_project_errors(self):
        _write(self.path,
               "project: nervi\nexposes: []\n"
               "consumes:\n  - capability: write-signal\n")
        self.assertIn("cap-consume-project",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    def test_consume_missing_capability_errors(self):
        _write(self.path,
               "project: nervi\nexposes: []\n"
               "consumes:\n  - project: farga\n")
        self.assertIn("cap-consume-capability",
                      _rules(lint_capabilities(self.path, "nervi"), "error"))

    # empty consumes is fine
    def test_empty_consumes_no_error(self):
        _write(self.path,
               "project: nervi\n"
               "exposes:\n  - id: nervi-mcp\n    kind: mcp-tool\n"
               "consumes: []\n")
        errors = [f for f in lint_capabilities(self.path, "nervi") if f.level == "error"]
        self.assertEqual(errors, [], f"empty consumes must not error: {errors}")


# ---------------------------------------------------------------------------
# Full project directory + generated dir tests
# ---------------------------------------------------------------------------

class FullProjectLintTests(unittest.TestCase):

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.generated = os.path.join(self.tmp, "generated")
        os.makedirs(self.generated, exist_ok=True)

    def test_valid_project_no_errors(self):
        project_dir, slug = _setup_project(self.generated, "nervi")
        _write_valid_project(project_dir, slug)
        findings, projects_checked = lint_generated(self.generated)
        errors = [f for f in findings if f.level == "error"]
        self.assertEqual(projects_checked, 1)
        self.assertEqual(errors, [], f"valid project produced errors: {errors}")

    def test_missing_domain_file_errors(self):
        project_dir, slug = _setup_project(self.generated, "nervi")
        _write_valid_project(project_dir, slug)
        os.remove(os.path.join(project_dir, "domains", "nervi.yaml"))
        findings, _ = lint_generated(self.generated)
        self.assertIn("domain-id-present", _rules(findings, "error"))

    def test_missing_capabilities_errors(self):
        project_dir, slug = _setup_project(self.generated, "nervi")
        _write_valid_project(project_dir, slug)
        os.remove(os.path.join(project_dir, "capabilities.yaml"))
        findings, _ = lint_generated(self.generated)
        self.assertIn("cap-project-present", _rules(findings, "error"))

    def test_multiple_projects_checked(self):
        for name in ("alpha", "beta"):
            project_dir, slug = _setup_project(self.generated, name)
            # Use nervi fixtures but fix slug references
            domain = VALID_DOMAIN.replace("domain/nervi", f"domain/{name}").replace(
                "repo: Nervi", f"repo: {name.capitalize()}")
            _write(os.path.join(project_dir, "domains", f"{name}.yaml"), domain)
            cap = VALID_CAPABILITIES.replace("project: nervi", f"project: {name}")
            _write(os.path.join(project_dir, "capabilities.yaml"), cap)
        _, projects_checked = lint_generated(self.generated)
        self.assertEqual(projects_checked, 2)

    def test_empty_generated_dir_no_error(self):
        findings, projects_checked = lint_generated(self.generated)
        self.assertEqual(projects_checked, 0)
        self.assertEqual(findings, [])

    def test_nonexistent_generated_dir_no_error(self):
        findings, projects_checked = lint_generated("/nonexistent/path")
        self.assertEqual(projects_checked, 0)
        self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
