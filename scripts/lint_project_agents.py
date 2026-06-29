#!/usr/bin/env python3
"""Structural lint for project-agent compositions (Fondament F-2).

Validates `kind: project-composition` YAML files (those instantiated under
`definitions/fondament/projects/`) against the composition schema. It is a
lightweight, dependency-light CI gate that runs without building the Rust
toolchain — complementary to `fondament check` (the in-process `run_fast`
lint that ships inside Caissa).

Rules
-----
error  yaml-parse              file is not valid YAML / not a mapping
error  kind-project-composition `kind` must be exactly "project-composition"
error  id-present              `id` must be present and non-empty
error  id-path-convention      `id` must equal the path under definitions/ minus ".yaml"
error  name-present            `name` must be present and non-empty
error  description-present     `description` must be present and non-empty
error  parts-present           `parts` must be a non-empty list
error  farga-part-project      each part with `source: farga` needs a non-empty `project`
warn   deconstructive-field    `deconstructive` is a spawn-time modifier, not a
                               composition field — serde drops it silently, so it
                               has no effect here.

`model` is optional. Warnings never fail the lint; errors exit non-zero.

The rules are intentionally stack-agnostic (Fondament also serves the Bosa
Properties stack): the id-path convention is derived from the definitions root,
not hardcoded to the Occitan layout.

Usage
-----
    python scripts/lint_project_agents.py [PROJECTS_DIR]

PROJECTS_DIR defaults to definitions/fondament/projects relative to the repo
root. The definitions root used for the id convention is auto-detected as the
nearest ancestor directory named "definitions".
"""

from __future__ import annotations

import os
import sys
from collections import namedtuple

import yaml

Finding = namedtuple("Finding", ["level", "rule", "path", "message"])

VALID_KIND = "project-composition"


def _detect_defs_root(projects_dir):
    """Walk up from projects_dir to the nearest ancestor named 'definitions'."""
    cur = os.path.abspath(projects_dir)
    while True:
        if os.path.basename(cur) == "definitions":
            return cur
        parent = os.path.dirname(cur)
        if parent == cur:
            # No 'definitions' ancestor: fall back to the parent of projects_dir.
            return os.path.dirname(os.path.abspath(projects_dir))
        cur = parent


def _expected_id(path, defs_root):
    rel = os.path.relpath(os.path.abspath(path), os.path.abspath(defs_root))
    rel = rel[:-len(".yaml")] if rel.endswith(".yaml") else rel
    if rel.endswith(".yml"):
        rel = rel[:-len(".yml")]
    return rel.replace(os.sep, "/")


def _missing(doc, key):
    val = doc.get(key)
    return val is None or (isinstance(val, str) and val.strip() == "")


def lint_file(path, defs_root):
    """Lint a single project-composition YAML file. Returns a list of Finding."""
    findings = []
    try:
        with open(path, "r", encoding="utf-8") as fh:
            doc = yaml.safe_load(fh)
    except yaml.YAMLError as exc:
        return [Finding("error", "yaml-parse", path, f"invalid YAML: {exc}")]

    if not isinstance(doc, dict):
        return [Finding("error", "yaml-parse", path,
                        "top-level YAML must be a mapping")]

    # Only enforce composition rules on composition files. A non-composition
    # kind sitting under projects/ is itself an error.
    if doc.get("kind") != VALID_KIND:
        findings.append(Finding(
            "error", "kind-project-composition", path,
            f"kind must be '{VALID_KIND}', got {doc.get('kind')!r}"))

    if _missing(doc, "id"):
        findings.append(Finding("error", "id-present", path,
                                "missing or empty 'id' field"))
    else:
        expected = _expected_id(path, defs_root)
        if doc["id"] != expected:
            findings.append(Finding(
                "error", "id-path-convention", path,
                f"id '{doc['id']}' does not match path convention '{expected}'"))

    if _missing(doc, "name"):
        findings.append(Finding("error", "name-present", path,
                                "missing or empty 'name' field"))

    if _missing(doc, "description"):
        findings.append(Finding("error", "description-present", path,
                                "missing or empty 'description' field"))

    parts = doc.get("parts")
    if not isinstance(parts, list) or len(parts) == 0:
        findings.append(Finding("error", "parts-present", path,
                                "'parts' must be a non-empty list"))
    else:
        for i, part in enumerate(parts):
            if isinstance(part, dict) and part.get("source") == "farga":
                project = part.get("project")
                if not isinstance(project, str) or project.strip() == "":
                    findings.append(Finding(
                        "error", "farga-part-project", path,
                        f"parts[{i}] with source: farga needs a non-empty 'project'"))

    if "deconstructive" in doc:
        findings.append(Finding(
            "warn", "deconstructive-field", path,
            "'deconstructive' is a spawn-time modifier, not a composition field; "
            "it has no effect here (serde drops it). Enable it via the "
            "CompositionAddress at dispatch time instead."))

    return findings


def lint_dir(projects_dir, defs_root=None):
    """Lint every *.yaml/*.yml file under projects_dir. Returns a list of Finding."""
    if defs_root is None:
        defs_root = _detect_defs_root(projects_dir)
    findings = []
    if not os.path.isdir(projects_dir):
        return findings
    for entry in sorted(os.listdir(projects_dir)):
        if entry.endswith((".yaml", ".yml")):
            findings.extend(lint_file(os.path.join(projects_dir, entry), defs_root))
    return findings


def main(argv):
    default_dir = os.path.join("definitions", "fondament", "projects")
    projects_dir = argv[1] if len(argv) > 1 else default_dir
    findings = lint_dir(projects_dir)

    errors = warns = 0
    for f in findings:
        marker = "FAIL" if f.level == "error" else "WARN"
        rel = os.path.relpath(f.path)
        print(f"{marker}  {rel} [{f.rule}]: {f.message}", file=sys.stderr)
        if f.level == "error":
            errors += 1
        else:
            warns += 1

    checked = (len([e for e in os.listdir(projects_dir)
                    if e.endswith((".yaml", ".yml"))])
               if os.path.isdir(projects_dir) else 0)
    print(f"project-agent lint: {checked} file(s), {errors} error(s), {warns} warning(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
