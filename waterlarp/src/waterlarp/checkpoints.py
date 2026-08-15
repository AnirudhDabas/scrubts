"""Canonical, self-identifying generation checkpoints."""

from __future__ import annotations

import json
import os
from collections.abc import Mapping
from pathlib import Path
from typing import Any

from waterlarp.manifests import canonical_json_bytes, canonical_sha256

CHECKPOINT_SCHEMA_VERSION = "1.0.0"
IDENTITY_FIELDS = (
    "experiment_spec_id",
    "sample_set_sha256",
    "model_identity_sha256",
    "tokenizer_identity_sha256",
    "generation_config_sha256",
)


def checkpoint_payload_sha256(checkpoint: Mapping[str, Any]) -> str:
    payload = {key: value for key, value in checkpoint.items() if key != "payload_sha256"}
    return canonical_sha256(payload)


def new_checkpoint(identity: Mapping[str, str]) -> dict[str, Any]:
    missing = set(IDENTITY_FIELDS) - identity.keys()
    if missing:
        raise ValueError(f"checkpoint identity missing fields: {', '.join(sorted(missing))}")
    checkpoint: dict[str, Any] = {
        "schema_version": CHECKPOINT_SCHEMA_VERSION,
        **{field: identity[field] for field in IDENTITY_FIELDS},
        "entries": [],
    }
    checkpoint["payload_sha256"] = checkpoint_payload_sha256(checkpoint)
    return checkpoint


def validate_checkpoint(
    checkpoint: Mapping[str, Any], expected_identity: Mapping[str, str]
) -> None:
    if checkpoint.get("schema_version") != CHECKPOINT_SCHEMA_VERSION:
        raise ValueError("unsupported generation checkpoint schema")
    for field in IDENTITY_FIELDS:
        if checkpoint.get(field) != expected_identity.get(field):
            raise ValueError(f"stale checkpoint identity mismatch: {field}")
    entries = checkpoint.get("entries")
    if not isinstance(entries, list):
        raise TypeError("checkpoint entries must be an array")
    keys: list[tuple[str, str, str, str]] = []
    for entry in entries:
        if not isinstance(entry, Mapping):
            raise TypeError("checkpoint entries must be objects")
        key = tuple(str(entry.get(field, "")) for field in ("task", "split", "sample_id", "kind"))
        if any(not value for value in key):
            raise ValueError("checkpoint entry identity is incomplete")
        keys.append(key)  # type: ignore[arg-type]
    if len(keys) != len(set(keys)):
        raise ValueError("checkpoint contains duplicate generation entries")
    expected_payload = checkpoint_payload_sha256(checkpoint)
    if checkpoint.get("payload_sha256") != expected_payload:
        raise ValueError("generation checkpoint payload checksum mismatch")


def load_checkpoint(path: Path, expected_identity: Mapping[str, str]) -> dict[str, Any]:
    raw = path.read_bytes()
    try:
        checkpoint = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError("generation checkpoint is not canonical JSON") from exc
    if not isinstance(checkpoint, dict):
        raise TypeError("generation checkpoint must be an object")
    if raw != canonical_json_bytes(checkpoint, terminal_newline=True):
        raise ValueError("generation checkpoint bytes are not canonical")
    validate_checkpoint(checkpoint, expected_identity)
    return checkpoint


def write_checkpoint(path: Path, checkpoint: dict[str, Any]) -> None:
    checkpoint["payload_sha256"] = checkpoint_payload_sha256(checkpoint)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_bytes(canonical_json_bytes(checkpoint, terminal_newline=True))
    os.replace(temporary, path)


def append_entry(path: Path, checkpoint: dict[str, Any], entry: Mapping[str, Any]) -> None:
    key = tuple(str(entry[field]) for field in ("task", "split", "sample_id", "kind"))
    existing = {
        tuple(str(item[field]) for field in ("task", "split", "sample_id", "kind"))
        for item in checkpoint["entries"]
    }
    if key in existing:
        raise ValueError("checkpoint entry already exists")
    checkpoint["entries"].append(dict(entry))
    checkpoint["entries"].sort(
        key=lambda item: tuple(item[field] for field in ("task", "split", "sample_id", "kind"))
    )
    write_checkpoint(path, checkpoint)
