"""Offline Draft 2020-12 validation for every canonical run object."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


def _schema_root() -> Path:
    return Path(__file__).resolve().parents[2] / "schemas"


def _registry() -> tuple[Registry[Any], dict[str, dict[str, Any]]]:
    registry: Registry[Any] = Registry()
    schemas: dict[str, dict[str, Any]] = {}
    for path in sorted(_schema_root().glob("*.schema.json")):
        schema = json.loads(path.read_text(encoding="utf-8"))
        schemas[path.name] = schema
        registry = registry.with_resource(schema["$id"], Resource.from_contents(schema))
    return registry, schemas


def validate_run_schemas(run_dir: Path) -> dict[str, int]:
    registry, schemas = _registry()
    manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
    Draft202012Validator(schemas["experiment-manifest.schema.json"], registry=registry).validate(
        manifest
    )
    examples = [
        json.loads(line)
        for line in (run_dir / "examples.jsonl").read_text(encoding="utf-8").splitlines()
        if line
    ]
    example_validator = Draft202012Validator(
        schemas["example-record.schema.json"], registry=registry
    )
    for example in examples:
        example_validator.validate(example)
    aggregates = json.loads((run_dir / "aggregate.json").read_text(encoding="utf-8"))
    aggregate_validator = Draft202012Validator(schemas["aggregate.schema.json"], registry=registry)
    for aggregate in aggregates:
        aggregate_validator.validate(aggregate)
    return {"manifest": 1, "examples": len(examples), "aggregates": len(aggregates)}
