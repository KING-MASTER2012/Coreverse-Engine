#!/usr/bin/env python3
"""Decide which parts of CI a pull request has to run.

Reads the rules in .github/ci-areas.txt, looks at the files the pull request
changes and writes `rust`, `cpp`, `format` and `ffi` (each true/false) to
$GITHUB_OUTPUT. Everything is true, i.e. nothing is skipped, when

  * the event is not a pull request (pushes, the nightly run, manual runs),
  * the list of changed files cannot be worked out,
  * a changed file matches no rule at all (a gap in the list must only cost time).

The crates that `ffi` (the only Rust crate C++ links) depends on are not in the
rules file: they are computed from the workspace's Cargo.toml files on every run,
so a new dependency of `ffi` is picked up by itself.

Usage in CI:  python3 .github/scripts/changed-areas.py
Local test:   python3 .github/scripts/changed-areas.py --files-from list.txt
"""

from __future__ import annotations

import argparse
import fnmatch
import os
import posixpath
import subprocess
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RULES_FILE = REPO / ".github" / "ci-areas.txt"
AREAS = ("rust", "cpp", "format", "ffi")
KINDS = AREAS + ("all", "ignore")
DEP_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def parse_rules(text: str) -> list[tuple[str, str]]:
    rules = []
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        if len(parts) != 2 or parts[0] not in KINDS:
            sys.exit(f"{RULES_FILE}:{number}: expected '<kind> <glob>' with kind in {KINDS}, got: {raw!r}")
        rules.append((parts[0], parts[1].strip()))
    return rules


def matches(pattern: str, path: str) -> bool:
    return fnmatch.fnmatchcase(path, pattern)


def ffi_closure(repo: Path) -> list[str]:
    """Directories of the `ffi` crate and of every workspace crate it depends on."""
    root = tomllib.loads((repo / "Cargo.toml").read_text(encoding="utf-8"))
    workspace = root.get("workspace", {})
    ws_deps = workspace.get("dependencies", {})

    packages: dict[str, dict] = {}
    for member in workspace.get("members", []):
        for directory in sorted(repo.glob(member)):
            manifest = directory / "Cargo.toml"
            if manifest.is_file():
                data = tomllib.loads(manifest.read_text(encoding="utf-8"))
                if "package" in data:
                    packages[directory.relative_to(repo).as_posix()] = data

    def path_deps(directory: str, data: dict) -> set[str]:
        tables = [data.get(section, {}) for section in DEP_SECTIONS]
        for target in data.get("target", {}).values():
            tables += [target.get(section, {}) for section in DEP_SECTIONS]
        found = set()
        for table in tables:
            for name, spec in table.items():
                if not isinstance(spec, dict):
                    continue
                if "path" in spec:
                    found.add(posixpath.normpath(posixpath.join(directory, spec["path"])))
                elif spec.get("workspace") and isinstance(ws_deps.get(name), dict) and "path" in ws_deps[name]:
                    found.add(posixpath.normpath(ws_deps[name]["path"]))
        return {d for d in found if d in packages}

    start = next((d for d, data in packages.items() if data["package"].get("name") == "ffi"), None)
    if start is None:
        # No crate called ffi: be conservative, treat every Rust crate as part of it.
        return ["engine/rust"]
    seen, stack = set(), [start]
    while stack:
        current = stack.pop()
        if current not in seen:
            seen.add(current)
            stack.extend(path_deps(current, packages[current]))
    return sorted(seen)


def changed_files(args: argparse.Namespace) -> list[str] | None:
    """Changed paths of the pull request, or None = "run everything"."""
    if args.files_from:
        return [l.strip() for l in Path(args.files_from).read_text(encoding="utf-8").splitlines() if l.strip()]
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        return None
    # On a pull request the checkout is the merge commit (parents: the base branch
    # tip and the head), so HEAD^1..HEAD is exactly what the pull request changes.
    # --no-renames lists the old path of a rename too.
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "HEAD^1", "HEAD"],
        cwd=REPO, capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"::warning::could not list the changed files ({result.stderr.strip()}); running everything")
        return None
    return [line for line in result.stdout.splitlines() if line]


def classify(files: list[str], rules: list[tuple[str, str]], closure: list[str]):
    area_rules = [r for r in rules if r[0] != "ignore"]
    # The crates ffi depends on matter for the C++ build and for the ffi jobs.
    for directory in closure:
        area_rules += [("cpp", directory + "/*"), ("ffi", directory + "/*")]
    ignore = [pattern for kind, pattern in rules if kind == "ignore"]

    flags = {area: False for area in AREAS}
    reasons: dict[str, list[str]] = {area: [] for area in AREAS}
    ignored, unclassified = [], []

    def raise_flag(area: str, path: str, why: str) -> None:
        flags[area] = True
        if len(reasons[area]) < 3:
            reasons[area].append(f"{path} ({why})")

    for path in files:
        if any(matches(p, path) for p in ignore):
            ignored.append(path)
            continue
        hit = False
        for kind, pattern in area_rules:
            if matches(pattern, path):
                hit = True
                for area in (AREAS if kind == "all" else (kind,)):
                    raise_flag(area, path, f"{kind} {pattern}")
        if not hit:
            unclassified.append(path)
            for area in AREAS:
                raise_flag(area, path, "no rule matches, so everything runs")
    return flags, reasons, ignored, unclassified


def publish(flags: dict[str, bool], summary: list[str]) -> None:
    lines = [f"{area}={'true' if flags[area] else 'false'}" for area in AREAS]
    print("\n".join(lines))
    if out := os.environ.get("GITHUB_OUTPUT"):
        with open(out, "a", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
    if path := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(path, "a", encoding="utf-8") as handle:
            handle.write("\n".join(summary) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--files-from", help="read the changed paths from this file instead of git (for testing)")
    args = parser.parse_args()

    rules = parse_rules(RULES_FILE.read_text(encoding="utf-8"))
    files = changed_files(args)

    if files is None:
        flags = {area: True for area in AREAS}
        publish(flags, ["### CI areas", "Not a pull request (or the changed files are unknown): everything runs."])
        return

    closure = ffi_closure(REPO)
    flags, reasons, ignored, unclassified = classify(files, rules, closure)

    summary = ["### CI areas", f"{len(files)} changed file(s), {len(ignored)} of them need no CI.", "",
               "| area | runs | because of |", "|---|---|---|"]
    for area in AREAS:
        why = "; ".join(f"`{r}`" for r in reasons[area]) or "-"
        summary.append(f"| {area} | {'yes' if flags[area] else 'no'} | {why} |")
    summary += ["", f"ffi closure (from Cargo.toml): {', '.join(f'`{d}`' for d in closure)}"]
    if unclassified:
        summary += ["", "Files without a rule (everything runs; add a rule to .github/ci-areas.txt):"]
        summary += [f"- `{p}`" for p in unclassified[:20]]
    publish(flags, summary)


if __name__ == "__main__":
    main()
