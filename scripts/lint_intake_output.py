#!/usr/bin/env python3
"""CI lint for Intake Agent YAML output (Fondament F-3).

Validates the files produced by the `intake-architect` agent when onboarding a
new project.  The expected layout under the target directory is::

    <project-slug>/
      domains/<project-slug>.yaml   # domain definition
      roles/<project-slug>-architect.yaml
      roles/<project-slug>-developer.yaml   # optional but validated when present
      capabilities.yaml

All files placed at ``generated/`` are validated by the CI job.  Run locally::

    python scripts/lint_intake_output.py [GENERATED_DIR]

GENERATED_DIR defaults to ``generated/`` relative to the repo root.

Rules
-----
Errors (exit non-zero):

  yaml-parse              File is not valid YAML or top-level is not a mapping.
  domain-id-present       domain.yaml: ``id`` must be present and non-empty.
  domain-id-format        domain.yaml: ``id`` must match ``domain/<slug>``.
  domain-kind             domain.yaml: ``kind`` must be ``domain``.
  domain-repo-present     domain.yaml: ``repo`` must be present and non-empty.
  domain-context-present  domain.yaml: ``context`` must be present and non-empty.
  role-id-present         role YAML: ``id`` must be present and non-empty.
  role-id-format          role YAML: ``id`` must match ``fondament/<slug>``.
  role-kind               role YAML: ``kind`` must be ``role``.
  role-model-present      role YAML: ``default_model`` must be present and non-empty.
  role-context-present    role YAML: ``context`` must be present and non-empty.
  role-tools-present      role YAML: ``tools`` must be a mapping with ``always_on`` list.
  cap-project-present     capabilities.yaml: ``project`` must be present and non-empty.
  cap-exposes-list        capabilities.yaml: ``exposes`` must be a list (may be empty).
  cap-consumes-list       capabilities.yaml: ``consumes`` must be a list (may be empty).
  cap-expose-id           capabilities.yaml exposes entry: ``id`` required.
  cap-expose-kind         capabilities.yaml exposes entry: ``kind`` required.
  cap-consume-project     capabilities.yaml consumes entry: ``project`` required.
  cap-consume-capability  capabilities.yaml consumes entry: ``capability`` required.

Warnings (do not fail):

  role-tools-farga  role YAML: architect/developer should include Farga read/write tools.
  cap-empty-exposes capabilities.yaml: ``exposes`` is empty — verify this is intentional.
"""

from __future__ import annotations

import os
import sys
from collections import namedtuple

import yaml

Finding = namedtuple("Finding", ["level", "rule", "path", "message"])


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _missing(doc: dict, key: str) -> bool:
    val = doc.get(key)
    return val is None or (isinstance(val, str) and val.strip() == "")


def _load(path: str) -> tuple[dict | None, list[Finding]]:
    """Parse YAML. Returns (doc, []) on success or (None, [Finding]) on error."""
    try:
        with open(path, "r", encoding="utf-8") as fh:
            doc = yaml.safe_load(fh)
    except yaml.YAMLError as exc:
        return None, [Finding("error", "yaml-parse", path, f"invalid YAML: {exc}")]
    if not isinstance(doc, dict):
        return None, [Finding("error", "yaml-parse", path,
                              "top-level YAML value must be a mapping")]
    return doc, []


# ---------------------------------------------------------------------------
# domain.yaml lint
# ---------------------------------------------------------------------------

def lint_domain(path: str, slug: str) -> list[Finding]:
    doc, findings = _load(path)
    if doc is None:
        return findings

    if doc.get("kind") != "domain":
        findings.append(Finding("error", "domain-kind", path,
                                f"kind must be 'domain', got {doc.get('kind')!r}"))

    if _missing(doc, "id"):
        findings.append(Finding("error", "domain-id-present", path,
                                "missing or empty 'id' field"))
    else:
        expected_id = f"domain/{slug}"
        if doc["id"] != expected_id:
            findings.append(Finding(
                "error", "domain-id-format", path,
                f"id '{doc['id']}' must be '{expected_id}' for slug '{slug}'"))

    if _missing(doc, "repo"):
        findings.append(Finding("error", "domain-repo-present", path,
                                "missing or empty 'repo' field"))

    if _missing(doc, "context"):
        findings.append(Finding("error", "domain-context-present", path,
                                "missing or empty 'context' field"))

    return findings


# ---------------------------------------------------------------------------
# roles/*.yaml lint
# ---------------------------------------------------------------------------

FARGA_TOOLS = {"farga-read-context", "farga-write-signal"}


def lint_role(path: str, slug: str) -> list[Finding]:
    doc, findings = _load(path)
    if doc is None:
        return findings

    role_name = os.path.splitext(os.path.basename(path))[0]  # e.g. nervi-architect

    if doc.get("kind") != "role":
        findings.append(Finding("error", "role-kind", path,
                                f"kind must be 'role', got {doc.get('kind')!r}"))

    if _missing(doc, "id"):
        findings.append(Finding("error", "role-id-present", path,
                                "missing or empty 'id' field"))
    else:
        expected_id = f"fondament/{role_name}"
        if doc["id"] != expected_id:
            findings.append(Finding(
                "error", "role-id-format", path,
                f"id '{doc['id']}' must be 'fondament/{role_name}' (derived from filename)"))

    if _missing(doc, "default_model"):
        findings.append(Finding("error", "role-model-present", path,
                                "missing or empty 'default_model' field"))

    if _missing(doc, "context"):
        findings.append(Finding("error", "role-context-present", path,
                                "missing or empty 'context' field"))

    tools = doc.get("tools")
    if not isinstance(tools, dict):
        findings.append(Finding("error", "role-tools-present", path,
                                "'tools' must be a mapping with an 'always_on' list"))
    else:
        always_on = tools.get("always_on")
        if not isinstance(always_on, list):
            findings.append(Finding("error", "role-tools-present", path,
                                    "'tools.always_on' must be a list"))
        else:
            # Warn if Farga read/write tools are absent
            tool_ids = {t.get("id") for t in always_on if isinstance(t, dict)}
            missing_farga = FARGA_TOOLS - tool_ids
            if missing_farga:
                findings.append(Finding(
                    "warn", "role-tools-farga", path,
                    f"role is missing Farga tools: {sorted(missing_farga)}; "
                    "add them under tools.always_on for context continuity"))

    return findings


# ---------------------------------------------------------------------------
# capabilities.yaml lint
# ---------------------------------------------------------------------------

def lint_capabilities(path: str, slug: str) -> list[Finding]:
    doc, findings = _load(path)
    if doc is None:
        return findings

    if _missing(doc, "project"):
        findings.append(Finding("error", "cap-project-present", path,
                                "missing or empty 'project' field"))
    else:
        if doc["project"] != slug:
            findings.append(Finding(
                "error", "cap-project-present", path,
                f"'project' is '{doc['project']}' but slug directory is '{slug}'"))

    exposes = doc.get("exposes")
    if exposes is None:
        # exposes key missing — default to empty list, no error (field is optional)
        exposes = []
    if not isinstance(exposes, list):
        findings.append(Finding("error", "cap-exposes-list", path,
                                "'exposes' must be a list"))
    else:
        if len(exposes) == 0:
            findings.append(Finding("warn", "cap-empty-exposes", path,
                                    "'exposes' is empty — verify this is intentional"))
        for i, entry in enumerate(exposes):
            if not isinstance(entry, dict):
                continue
            if not entry.get("id"):
                findings.append(Finding("error", "cap-expose-id", path,
                                        f"exposes[{i}]: 'id' is required"))
            if not entry.get("kind"):
                findings.append(Finding("error", "cap-expose-kind", path,
                                        f"exposes[{i}]: 'kind' is required"))

    consumes = doc.get("consumes")
    if consumes is None:
        consumes = []
    if not isinstance(consumes, list):
        findings.append(Finding("error", "cap-consumes-list", path,
                                "'consumes' must be a list"))
    else:
        for i, entry in enumerate(consumes):
            if not isinstance(entry, dict):
                continue
            if not entry.get("project"):
                findings.append(Finding("error", "cap-consume-project", path,
                                        f"consumes[{i}]: 'project' is required"))
            if not entry.get("capability"):
                findings.append(Finding("error", "cap-consume-capability", path,
                                        f"consumes[{i}]: 'capability' is required"))

    return findings


# ---------------------------------------------------------------------------
# Per-project-slug directory lint
# ---------------------------------------------------------------------------

def lint_project_dir(project_dir: str) -> list[Finding]:
    """Lint one project output directory (named by its slug)."""
    slug = os.path.basename(os.path.abspath(project_dir))
    findings: list[Finding] = []

    # --- domains/<slug>.yaml ---
    domain_path = os.path.join(project_dir, "domains", f"{slug}.yaml")
    if os.path.isfile(domain_path):
        findings.extend(lint_domain(domain_path, slug))
    else:
        findings.append(Finding("error", "domain-id-present", project_dir,
                                f"missing domain file: domains/{slug}.yaml"))

    # --- roles/*.yaml ---
    roles_dir = os.path.join(project_dir, "roles")
    if os.path.isdir(roles_dir):
        for entry in sorted(os.listdir(roles_dir)):
            if entry.endswith((".yaml", ".yml")):
                findings.extend(lint_role(os.path.join(roles_dir, entry), slug))
    # A project with no roles at all is suspicious but not necessarily wrong —
    # the intake format only requires architect and developer stubs, but we don't
    # fail on their absence because intake may not always produce both.

    # --- capabilities.yaml ---
    cap_path = os.path.join(project_dir, "capabilities.yaml")
    if os.path.isfile(cap_path):
        findings.extend(lint_capabilities(cap_path, slug))
    else:
        findings.append(Finding("error", "cap-project-present", project_dir,
                                "missing capabilities.yaml"))

    return findings


# ---------------------------------------------------------------------------
# Top-level: walk generated/ for project slug subdirectories
# ---------------------------------------------------------------------------

def lint_generated(generated_dir: str) -> tuple[list[Finding], int]:
    """Lint all project-slug subdirectories under generated_dir.

    Returns (findings, projects_checked).
    """
    findings: list[Finding] = []
    projects_checked = 0
    if not os.path.isdir(generated_dir):
        return findings, 0

    for entry in sorted(os.listdir(generated_dir)):
        entry_path = os.path.join(generated_dir, entry)
        if os.path.isdir(entry_path):
            findings.extend(lint_project_dir(entry_path))
            projects_checked += 1

    return findings, projects_checked


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main(argv: list[str]) -> int:
    default_dir = "generated"
    generated_dir = argv[1] if len(argv) > 1 else default_dir

    findings, projects_checked = lint_generated(generated_dir)

    errors = warns = 0
    for f in findings:
        marker = "FAIL" if f.level == "error" else "WARN"
        try:
            rel = os.path.relpath(f.path)
        except ValueError:
            rel = f.path
        print(f"{marker}  {rel} [{f.rule}]: {f.message}", file=sys.stderr)
        if f.level == "error":
            errors += 1
        else:
            warns += 1

    print(
        f"intake-output lint: {projects_checked} project(s), "
        f"{errors} error(s), {warns} warning(s)"
    )
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
