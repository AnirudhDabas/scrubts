#!/usr/bin/env python3
"""Compare actual Windows, Linux, and macOS canonical report digests."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
from typing import Any


PLATFORMS = ("linux", "macos", "windows")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
GIT_REVISION = re.compile(r"^[0-9a-f]{40}$")


def load_results(directory: Path) -> dict[str, dict[str, Any]]:
    results: dict[str, dict[str, Any]] = {}
    for path in directory.rglob("determinism-platform.json"):
        document = json.loads(path.read_text(encoding="utf-8"))
        if set(document) != {"schema_version", "project_revision", "platform", "fixtures"}:
            raise ValueError(f"invalid platform result keys: {path}")
        if document["schema_version"] != "0.1" or not GIT_REVISION.fullmatch(document["project_revision"]):
            raise ValueError(f"invalid platform result identity: {path}")
        platform = document["platform"]
        if platform not in PLATFORMS or platform in results:
            raise ValueError(f"missing, duplicate, or unknown platform: {platform}")
        if not isinstance(document["fixtures"], list) or not document["fixtures"]:
            raise ValueError(f"empty fixture result: {path}")
        results[platform] = document
    if set(results) != set(PLATFORMS):
        raise ValueError(f"expected all platforms {PLATFORMS}, got {sorted(results)}")
    return results


def compare(results: dict[str, dict[str, Any]]) -> tuple[dict[str, Any], bool]:
    revisions = {document["project_revision"] for document in results.values()}
    if len(revisions) != 1:
        raise ValueError("platform results were not generated from one project revision")

    indexed: dict[str, dict[str, dict[str, Any]]] = {}
    for platform, document in results.items():
        entries: dict[str, dict[str, Any]] = {}
        for fixture in document["fixtures"]:
            required = {"fixture_id", "input_sha256", "expected_capability", "semantic_report_sha256"}
            if set(fixture) != required:
                raise ValueError(f"invalid fixture fields on {platform}")
            if fixture["fixture_id"] in entries:
                raise ValueError(f"duplicate fixture ID on {platform}: {fixture['fixture_id']}")
            if not SHA256.fullmatch(fixture["input_sha256"]) or not SHA256.fullmatch(fixture["semantic_report_sha256"]):
                raise ValueError(f"invalid digest on {platform}: {fixture['fixture_id']}")
            entries[fixture["fixture_id"]] = fixture
        indexed[platform] = entries

    fixture_sets = {tuple(sorted(entries)) for entries in indexed.values()}
    if len(fixture_sets) != 1:
        raise ValueError("platform fixture ID sets differ")

    fixtures = []
    all_equal = True
    for fixture_id in next(iter(fixture_sets)):
        entries = {platform: indexed[platform][fixture_id] for platform in PLATFORMS}
        input_digests = {entry["input_sha256"] for entry in entries.values()}
        capabilities = {entry["expected_capability"] for entry in entries.values()}
        if len(input_digests) != 1:
            raise ValueError(f"input bytes differ before semantic comparison: {fixture_id}")
        if len(capabilities) != 1:
            raise ValueError(f"expected capabilities differ: {fixture_id}")
        semantic = {platform: entries[platform]["semantic_report_sha256"] for platform in PLATFORMS}
        equal = len(set(semantic.values())) == 1
        all_equal = all_equal and equal
        fixtures.append(
            {
                "fixture_id": fixture_id,
                "input_sha256": next(iter(input_digests)),
                "expected_capability": next(iter(capabilities)),
                "semantic_digests": semantic,
                "equal": equal,
            }
        )
    return (
        {
            "schema_version": "0.1",
            "project_revision": next(iter(revisions)),
            "platforms": list(PLATFORMS),
            "fixtures": fixtures,
            "equality_status": "ESTABLISHED" if all_equal else "NOT_EQUAL",
            "allowed_capability_differences": [],
        },
        all_equal,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    matrix, equal = compare(load_results(arguments.input))
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(matrix, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(matrix["equality_status"])
    return 0 if equal else 1


if __name__ == "__main__":
    raise SystemExit(main())
