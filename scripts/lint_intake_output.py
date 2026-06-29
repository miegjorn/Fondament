#!/usr/bin/env python3
"""Structural lint for Intake Agent YAML output (Fondament F-3).

Validates the YAML bundle produced by the `intake-architect` agent against the
canonical output format (`docs/intake-output-format.md`). The goal is to fail
explicitly on malformed drafts *before* human review, with messages that name
the offending field and the constraint it violated.

Like the F-2 project-agent lint (`lint_project_agents.py`), this is a
dependency-light CI gate: it needs only PyYAML, runs without building the Rust
toolchain, and is stack-agnostic (Fondament also serves the Bosa Properties
stack), so nothing here is hardcoded to the Occitan layout.

An intake bundle is a directory shaped like:

    <project-slug>/
      domains/<project-slug>.yaml   # kind: domain
      roles/<role>.yaml             # kind: role  (architect, developer, ...)
      capabilities.yaml             # project / exposes / consumes
      README.md                     # not linted here (prose)

Files are dispatched by their `kind` field (domain / role); a file named
`capabilities.yaml` / `.yml` is validated as a capabilities contract. Any other
YAML file in the bundle is reported as `unknown-kind`.

Rules
-----
error  yaml-parse                file is not valid YAML / not a mapping
error  unknown-kind              YAML file is neither domain, role nor capabilities

domain (kind: domain)
error  domain-kind               `kind` must be exactly "domain"
error  domain-id-present         `id` must be present and non-empty
error  domain-id-convention      `id` must equal "domain/<filename>"
error  domain-repo-present       `repo` must be present and non-empty
error  domain-context-present    `context` must be present and non-empty

role (kind: role)
error  role-kind                 `kind` must be exactly "role"
error  role-id-present           `id` must be present and non-empty
error  role-id-convention        `id` must equal "fondament/<filename>"
error  role-default-model-valid  `default_model` must be a known Claude model id
error  role-context-present      `context` must be present and non-empty
error  role-tools-present        `tools.always_on` must be a non-empty list
error  role-tool-fields          each tool needs `id`+`kind`; mcp→server+tool, native→tool
error  role-jit-list             `tools.jit` must be a list when present

capabilities (capabilities.yaml)
error  capabilities-project-present  `project` must be present and non-empty
error  capabilities-exposes-list     `exposes` must be a list when present
error  capabilities-exposes-entry    each exposes entry needs id/kind/description
error  capabilities-exposes-kind     exposes `kind` must be a known capability kind
error  capabilities-consumes-list    `consumes` must be a list when present
error  capabilities-consumes-entry   each consumes entry needs project/capability
warn   capabilities-required-bool    consumes `required` should be a boolean

Warnings never fail the lint; errors exit non-zero.

Usage
-----
    python scripts/lint_intake_output.py [BUNDLE_DIR]

BUNDLE_DIR defaults to `generated/` (the intake staging directory). The script
walks it recursively, so it accepts either a single bundle or a parent holding
several. A missing directory is reported as zero files checked (exit 0), which
keeps the gate quiet until intake output actually lands.
"""

from __future__ import annotations

import os
import sys
from collections import namedtuple

import yaml

Finding = namedtuple("Finding", ["level", "rule", "path", "message"])

# Matches the fast lint's `valid-model-id` rule (see README "Lint System").
VALID_MODELS = {
    "claude-haiku-4-5-20251001",
    "claude-sonnet-4-6",
    "claude-opus-4-8",
    "claude-fable-5",
}

# Capability kinds enumerated in docs/intake-output-format.md.
VALID_CAPABILITY_KINDS = {
    "http-api",
    "mcp-tool",
    "matrix-identity",
    "helm-chart",
    "library",
}


def _missing(doc, key):
    val = doc.get(key)
    return val is None or (isinstance(val, str) and val.strip() == "")


def _basename_no_ext(path):
    name = os.path.basename(path)
    for ext in (".yaml", ".yml"):
        if name.endswith(ext):
            return name[: -len(ext)]
    return name


def _lint_tool(entry, idx, path, findings):
    """Validate a single tool entry inside tools.always_on / tools.jit."""
    if not isinstance(entry, dict):
        findings.append(Finding(
            "error", "role-tool-fields", path,
            f"tools entry [{idx}] must be a mapping, got {type(entry).__name__}"))
        return
    if _missing(entry, "id"):
        findings.append(Finding(
            "error", "role-tool-fields", path,
            f"tools entry [{idx}] is missing required field 'id'"))
    kind = entry.get("kind")
    if _missing(entry, "kind"):
        findings.append(Finding(
            "error", "role-tool-fields", path,
            f"tools entry [{idx}] is missing required field 'kind'"))
    elif kind == "mcp":
        for field in ("server", "tool"):
            if _missing(entry, field):
                findings.append(Finding(
                    "error", "role-tool-fields", path,
                    f"tools entry [{idx}] kind 'mcp' requires '{field}'"))
    elif kind == "native":
        if _missing(entry, "tool"):
            findings.append(Finding(
                "error", "role-tool-fields", path,
                f"tools entry [{idx}] kind 'native' requires 'tool'"))


def lint_domain(doc, path):
    findings = []
    if doc.get("kind") != "domain":
        findings.append(Finding(
            "error", "domain-kind", path,
            f"field 'kind' must be 'domain', got {doc.get('kind')!r}"))
    if _missing(doc, "id"):
        findings.append(Finding("error", "domain-id-present", path,
                                "field 'id' is missing or empty"))
    else:
        expected = f"domain/{_basename_no_ext(path)}"
        if doc["id"] != expected:
            findings.append(Finding(
                "error", "domain-id-convention", path,
                f"field 'id' is {doc['id']!r}, expected {expected!r} "
                "(must be 'domain/<filename>')"))
    if _missing(doc, "repo"):
        findings.append(Finding("error", "domain-repo-present", path,
                                "field 'repo' is missing or empty"))
    if _missing(doc, "context"):
        findings.append(Finding("error", "domain-context-present", path,
                                "field 'context' is missing or empty"))
    return findings


def lint_role(doc, path):
    findings = []
    if doc.get("kind") != "role":
        findings.append(Finding(
            "error", "role-kind", path,
            f"field 'kind' must be 'role', got {doc.get('kind')!r}"))
    if _missing(doc, "id"):
        findings.append(Finding("error", "role-id-present", path,
                                "field 'id' is missing or empty"))
    else:
        expected = f"fondament/{_basename_no_ext(path)}"
        if doc["id"] != expected:
            findings.append(Finding(
                "error", "role-id-convention", path,
                f"field 'id' is {doc['id']!r}, expected {expected!r} "
                "(must be 'fondament/<filename>')"))
    model = doc.get("default_model")
    if _missing(doc, "default_model"):
        findings.append(Finding("error", "role-default-model-valid", path,
                                "field 'default_model' is missing or empty"))
    elif model not in VALID_MODELS:
        findings.append(Finding(
            "error", "role-default-model-valid", path,
            f"field 'default_model' is {model!r}, must be one of "
            f"{sorted(VALID_MODELS)}"))
    if _missing(doc, "context"):
        findings.append(Finding("error", "role-context-present", path,
                                "field 'context' is missing or empty"))

    tools = doc.get("tools")
    if not isinstance(tools, dict):
        findings.append(Finding("error", "role-tools-present", path,
                                "field 'tools' must be a mapping with 'always_on'"))
    else:
        always_on = tools.get("always_on")
        if not isinstance(always_on, list) or len(always_on) == 0:
            findings.append(Finding(
                "error", "role-tools-present", path,
                "field 'tools.always_on' must be a non-empty list"))
        else:
            for i, entry in enumerate(always_on):
                _lint_tool(entry, i, path, findings)
        jit = tools.get("jit")
        if jit is not None and not isinstance(jit, list):
            findings.append(Finding(
                "error", "role-jit-list", path,
                "field 'tools.jit' must be a list when present"))
        elif isinstance(jit, list):
            for i, entry in enumerate(jit):
                _lint_tool(entry, i, path, findings)
    return findings


def lint_capabilities(doc, path):
    findings = []
    if _missing(doc, "project"):
        findings.append(Finding("error", "capabilities-project-present", path,
                                "field 'project' is missing or empty"))

    exposes = doc.get("exposes")
    if exposes is not None and not isinstance(exposes, list):
        findings.append(Finding("error", "capabilities-exposes-list", path,
                                "field 'exposes' must be a list when present"))
    elif isinstance(exposes, list):
        for i, entry in enumerate(exposes):
            if not isinstance(entry, dict):
                findings.append(Finding(
                    "error", "capabilities-exposes-entry", path,
                    f"exposes[{i}] must be a mapping"))
                continue
            for field in ("id", "kind", "description"):
                if _missing(entry, field):
                    findings.append(Finding(
                        "error", "capabilities-exposes-entry", path,
                        f"exposes[{i}] is missing required field '{field}'"))
            kind = entry.get("kind")
            if kind is not None and kind not in VALID_CAPABILITY_KINDS:
                findings.append(Finding(
                    "error", "capabilities-exposes-kind", path,
                    f"exposes[{i}] kind {kind!r} must be one of "
                    f"{sorted(VALID_CAPABILITY_KINDS)}"))

    consumes = doc.get("consumes")
    if consumes is not None and not isinstance(consumes, list):
        findings.append(Finding("error", "capabilities-consumes-list", path,
                                "field 'consumes' must be a list when present"))
    elif isinstance(consumes, list):
        for i, entry in enumerate(consumes):
            if not isinstance(entry, dict):
                findings.append(Finding(
                    "error", "capabilities-consumes-entry", path,
                    f"consumes[{i}] must be a mapping"))
                continue
            for field in ("project", "capability"):
                if _missing(entry, field):
                    findings.append(Finding(
                        "error", "capabilities-consumes-entry", path,
                        f"consumes[{i}] is missing required field '{field}'"))
            if "required" in entry and not isinstance(entry["required"], bool):
                findings.append(Finding(
                    "warn", "capabilities-required-bool", path,
                    f"consumes[{i}] field 'required' should be a boolean, "
                    f"got {type(entry['required']).__name__}"))
    return findings


def lint_file(path):
    """Lint a single intake YAML file, dispatching on kind / filename."""
    try:
        with open(path, "r", encoding="utf-8") as fh:
            doc = yaml.safe_load(fh)
    except yaml.YAMLError as exc:
        return [Finding("error", "yaml-parse", path, f"invalid YAML: {exc}")]

    if not isinstance(doc, dict):
        return [Finding("error", "yaml-parse", path,
                        "top-level YAML must be a mapping")]

    kind = doc.get("kind")
    if kind == "domain":
        return lint_domain(doc, path)
    if kind == "role":
        return lint_role(doc, path)
    if _basename_no_ext(path) == "capabilities":
        return lint_capabilities(doc, path)

    return [Finding(
        "error", "unknown-kind", path,
        f"cannot classify file: kind={kind!r} and name is not 'capabilities' "
        "(expected kind 'domain'/'role' or a capabilities.yaml)")]


def _yaml_files(bundle_dir):
    for root, _dirs, files in os.walk(bundle_dir):
        for entry in sorted(files):
            if entry.endswith((".yaml", ".yml")):
                yield os.path.join(root, entry)


def lint_dir(bundle_dir):
    """Lint every *.yaml/*.yml under bundle_dir (recursive). Returns Findings."""
    findings = []
    if not os.path.isdir(bundle_dir):
        return findings
    for path in sorted(_yaml_files(bundle_dir)):
        findings.extend(lint_file(path))
    return findings


def main(argv):
    bundle_dir = argv[1] if len(argv) > 1 else "generated"
    findings = lint_dir(bundle_dir)

    errors = warns = 0
    for f in findings:
        marker = "FAIL" if f.level == "error" else "WARN"
        rel = os.path.relpath(f.path)
        print(f"{marker}  {rel} [{f.rule}]: {f.message}", file=sys.stderr)
        if f.level == "error":
            errors += 1
        else:
            warns += 1

    checked = sum(1 for _ in _yaml_files(bundle_dir)) if os.path.isdir(bundle_dir) else 0
    print(f"intake-output lint: {checked} file(s), {errors} error(s), {warns} warning(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
