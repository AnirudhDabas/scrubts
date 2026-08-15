"""Offline source-ledger verification for canonical runs."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

import yaml


def load_source_ledger(path: Path) -> dict[str, Any]:
    document = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict) or not isinstance(document.get("sources"), list):
        raise ValueError("source ledger must contain a sources list")
    ids = [source.get("id") for source in document["sources"] if isinstance(source, dict)]
    if any(not isinstance(source_id, str) or not source_id for source_id in ids):
        raise ValueError("every source must have a non-empty string id")
    if len(ids) != len(set(ids)):
        raise ValueError("source IDs must be unique")
    return document


def verify_artifacts(
    ledger: dict[str, Any], checkouts: dict[str, Path] | None
) -> list[dict[str, str]]:
    results: list[dict[str, str]] = []
    for source in ledger["sources"]:
        hashes = source.get("artifact_sha256", {})
        for relative, expected in hashes.items():
            if checkouts is None or source["id"] not in checkouts:
                results.append(
                    {"source_id": source["id"], "path": relative, "status": "IDENTITY_RECORDED"}
                )
                continue
            candidate = checkouts[source["id"]] / relative
            if not candidate.is_file():
                results.append(
                    {"source_id": source["id"], "path": relative, "status": "CACHE_MISSING"}
                )
                continue
            actual = hashlib.sha256(candidate.read_bytes()).hexdigest()
            results.append(
                {
                    "source_id": source["id"],
                    "path": relative,
                    "status": "VERIFIED" if actual == expected else "HASH_MISMATCH",
                }
            )
    return results
