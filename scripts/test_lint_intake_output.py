"""Tests for the Intake Agent output CI lint (Fondament F-3).

Run with: python -m unittest scripts/test_lint_intake_output.py
(or: python scripts/test_lint_intake_output.py)

These tests are the F-3 acceptance criteria. The invalid-file cases were written
*before* the lint logic existed (TDD): each constructs a deliberately broken
intake YAML and asserts the lint flags it with the expected rule and a message
naming the offending field. The happy-path cases assert valid bundles are clean.
"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from lint_intake_output import (  # noqa: E402
    lint_capabilities,
    lint_dir,
    lint_domain,
    lint_file,
    lint_role,
)


VALID_DOMAIN = """\
id: domain/nervi
kind: domain
repo: nervi
default_facet: architect
context: |
  Nèrvi is the async subscription fabric of the Occitan stack.

  ## Design constraints
  - Substrat: NATS JetStream.
"""

VALID_ROLE = """\
id: fondament/nervi-architect
kind: role
default_model: claude-sonnet-4-6
context: |
  You are the architect agent for Nèrvi.
tools:
  always_on:
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
  jit: []
skills:
  - superpowers:test-driven-development
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


def _write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(content)


def _rules(findings, level=None):
    return {f.rule for f in findings if level is None or f.level == level}


def _load(content):
    import yaml
    return yaml.safe_load(content)


class DomainLintTests(unittest.TestCase):
    def test_valid_domain_has_no_errors(self):
        f = lint_domain(_load(VALID_DOMAIN), "domains/nervi.yaml")
        self.assertEqual([x for x in f if x.level == "error"], [])

    def test_wrong_kind_errors(self):
        doc = _load(VALID_DOMAIN.replace("kind: domain", "kind: role"))
        self.assertIn("domain-kind", _rules(lint_domain(doc, "domains/nervi.yaml"), "error"))

    def test_missing_id_errors(self):
        doc = _load(VALID_DOMAIN.replace("id: domain/nervi\n", ""))
        self.assertIn("domain-id-present", _rules(lint_domain(doc, "domains/nervi.yaml"), "error"))

    def test_id_convention_mismatch_errors(self):
        doc = _load(VALID_DOMAIN.replace("id: domain/nervi", "id: domain/wrong"))
        self.assertIn("domain-id-convention", _rules(lint_domain(doc, "domains/nervi.yaml"), "error"))

    def test_missing_repo_errors(self):
        doc = _load(VALID_DOMAIN.replace("repo: nervi\n", ""))
        self.assertIn("domain-repo-present", _rules(lint_domain(doc, "domains/nervi.yaml"), "error"))

    def test_empty_context_errors(self):
        doc = _load(VALID_DOMAIN)
        doc["context"] = "   "
        self.assertIn("domain-context-present", _rules(lint_domain(doc, "domains/nervi.yaml"), "error"))


class RoleLintTests(unittest.TestCase):
    def test_valid_role_has_no_errors(self):
        f = lint_role(_load(VALID_ROLE), "roles/nervi-architect.yaml")
        self.assertEqual([x for x in f if x.level == "error"], [])

    def test_wrong_kind_errors(self):
        doc = _load(VALID_ROLE.replace("kind: role", "kind: domain"))
        self.assertIn("role-kind", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_id_convention_mismatch_errors(self):
        doc = _load(VALID_ROLE.replace("fondament/nervi-architect", "fondament/wrong"))
        self.assertIn("role-id-convention", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_invalid_model_errors(self):
        doc = _load(VALID_ROLE.replace("claude-sonnet-4-6", "gpt-4"))
        self.assertIn("role-default-model-valid", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_missing_model_errors(self):
        doc = _load(VALID_ROLE.replace("default_model: claude-sonnet-4-6\n", ""))
        self.assertIn("role-default-model-valid", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_empty_tools_errors(self):
        doc = _load(VALID_ROLE)
        doc["tools"] = {"always_on": [], "jit": []}
        self.assertIn("role-tools-present", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_missing_tools_errors(self):
        doc = _load(VALID_ROLE)
        del doc["tools"]
        self.assertIn("role-tools-present", _rules(lint_role(doc, "roles/nervi-architect.yaml"), "error"))

    def test_mcp_tool_missing_server_errors(self):
        bad = VALID_ROLE.replace("      server: farga\n", "")
        self.assertIn("role-tool-fields", _rules(lint_role(_load(bad), "roles/nervi-architect.yaml"), "error"))

    def test_tool_missing_kind_errors(self):
        bad = VALID_ROLE.replace("      kind: mcp\n", "")
        self.assertIn("role-tool-fields", _rules(lint_role(_load(bad), "roles/nervi-architect.yaml"), "error"))


class CapabilitiesLintTests(unittest.TestCase):
    def test_valid_capabilities_has_no_errors(self):
        f = lint_capabilities(_load(VALID_CAPABILITIES), "capabilities.yaml")
        self.assertEqual([x for x in f if x.level == "error"], [])

    def test_missing_project_errors(self):
        doc = _load(VALID_CAPABILITIES.replace("project: nervi\n", ""))
        self.assertIn("capabilities-project-present", _rules(lint_capabilities(doc, "capabilities.yaml"), "error"))

    def test_exposes_entry_missing_field_errors(self):
        bad = VALID_CAPABILITIES.replace("    description: MCP server exposing nervi_publish and nervi_subscribe\n", "")
        self.assertIn("capabilities-exposes-entry", _rules(lint_capabilities(_load(bad), "capabilities.yaml"), "error"))

    def test_exposes_bad_kind_errors(self):
        doc = _load(VALID_CAPABILITIES.replace("kind: mcp-tool", "kind: nonsense"))
        self.assertIn("capabilities-exposes-kind", _rules(lint_capabilities(doc, "capabilities.yaml"), "error"))

    def test_consumes_entry_missing_field_errors(self):
        bad = VALID_CAPABILITIES.replace("    capability: write-signal\n", "")
        self.assertIn("capabilities-consumes-entry", _rules(lint_capabilities(_load(bad), "capabilities.yaml"), "error"))

    def test_consumes_required_non_bool_warns(self):
        doc = _load(VALID_CAPABILITIES.replace("required: false", "required: maybe"))
        findings = lint_capabilities(doc, "capabilities.yaml")
        self.assertIn("capabilities-required-bool", _rules(findings, "warn"))
        self.assertNotIn("capabilities-required-bool", _rules(findings, "error"))


class FileDispatchTests(unittest.TestCase):
    def test_invalid_yaml_errors(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "domains", "x.yaml")
            _write(p, "id: x\n  : : bad\n :\n")
            self.assertIn("yaml-parse", _rules(lint_file(p), "error"))

    def test_unknown_kind_errors(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "mystery.yaml")
            _write(p, "id: something\nkind: discipline\n")
            self.assertIn("unknown-kind", _rules(lint_file(p), "error"))


class BundleLintTests(unittest.TestCase):
    """End-to-end: a full valid bundle is clean; a malformed one fails."""

    def _bundle(self, tmp, domain=VALID_DOMAIN, role=VALID_ROLE, caps=VALID_CAPABILITIES):
        root = os.path.join(tmp, "nervi")
        _write(os.path.join(root, "domains", "nervi.yaml"), domain)
        _write(os.path.join(root, "roles", "nervi-architect.yaml"), role)
        _write(os.path.join(root, "capabilities.yaml"), caps)
        return root

    def test_valid_bundle_is_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._bundle(tmp)
            errors = [f for f in lint_dir(root) if f.level == "error"]
            self.assertEqual(errors, [], f"valid bundle produced errors: {errors}")

    def test_malformed_bundle_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._bundle(tmp, role=VALID_ROLE.replace("claude-sonnet-4-6", "gpt-4"))
            errors = [f for f in lint_dir(root) if f.level == "error"]
            self.assertTrue(errors, "malformed bundle must produce at least one error")
            self.assertIn("role-default-model-valid", {f.rule for f in errors})


if __name__ == "__main__":
    unittest.main()
