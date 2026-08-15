"""Canonical experiment, run, sample-set, and artifact identities."""

from __future__ import annotations

import hashlib
import json
import math
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


def _validate_json(value: Any, path: str = "$") -> None:
    if value is None or isinstance(value, (bool, str, int)):
        return
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"non-finite number at {path}")
        return
    if isinstance(value, (list, tuple)):
        for index, item in enumerate(value):
            _validate_json(item, f"{path}[{index}]")
        return
    if isinstance(value, Mapping):
        for key, item in value.items():
            if not isinstance(key, str):
                raise TypeError(f"non-string object key at {path}")
            _validate_json(item, f"{path}.{key}")
        return
    raise TypeError(f"unsupported canonical JSON value {type(value).__name__} at {path}")


def canonical_json_bytes(value: Any, *, terminal_newline: bool = False) -> bytes:
    _validate_json(value)
    encoded = json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return encoded + (b"\n" if terminal_newline else b"")


def sha256_hex(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_sha256(value: Any) -> str:
    return sha256_hex(canonical_json_bytes(value))


def experiment_specification_id(specification: Mapping[str, Any]) -> str:
    return f"wlrs1-{canonical_sha256(dict(specification))[:24]}"


def execution_run_id(experiment_spec_id: str, checkpoint_payload_sha256: str) -> str:
    identity = {
        "experiment_spec_id": experiment_spec_id,
        "checkpoint_payload_sha256": checkpoint_payload_sha256,
    }
    return f"wlrp1-{canonical_sha256(identity)[:24]}"


def specification_run_id(specification: Mapping[str, Any]) -> str:
    """Compatibility name for tests/callers; now returns the specification ID."""

    return experiment_specification_id(specification)


def artifact_set_id(scientific_artifact_sha256: Mapping[str, str]) -> str:
    return f"wlra1-{canonical_sha256(dict(scientific_artifact_sha256))[:24]}"


def hash_ids(ids: list[str] | tuple[str, ...]) -> str:
    if len(ids) != len(set(ids)):
        raise ValueError("sample IDs must be unique")
    return canonical_sha256(list(ids))


def validate_sample_sets(sample_sets: Mapping[str, Any]) -> None:
    if not sample_sets:
        raise ValueError("sample_sets must not be empty")
    all_ids: list[str] = []
    for task, descriptor in sample_sets.items():
        if not isinstance(task, str) or not isinstance(descriptor, Mapping):
            raise TypeError("sample-set task entries must be objects")
        required = {
            "dataset",
            "dataset_revision",
            "source_split",
            "prompt_template_sha256",
            "splits",
        }
        if not required <= descriptor.keys():
            raise ValueError(f"sample set {task} lacks source/reconstruction identity")
        splits = descriptor["splits"]
        if not isinstance(splits, Mapping) or set(splits) != {
            "generation",
            "calibration",
            "test",
        }:
            raise ValueError("each task must preserve generation/calibration/test member arrays")
        task_ids: list[str] = []
        task_rows: list[tuple[int, str]] = []
        for split_name in ("generation", "calibration", "test"):
            members = splits[split_name]
            if not isinstance(members, list) or not members:
                raise ValueError("every split must contain at least one exact member")
            split_ids: list[str] = []
            for member in members:
                if not isinstance(member, Mapping):
                    raise TypeError("sample-set members must be objects")
                if set(member) != {"sample_id", "row_index", "row_sha256"}:
                    raise ValueError("sample members require exact ID, row mapping, and row hash")
                sample_id = member["sample_id"]
                row_index = member["row_index"]
                row_sha256 = member["row_sha256"]
                if not isinstance(sample_id, str) or not sample_id:
                    raise ValueError("sample_id must be a non-empty string")
                if not isinstance(row_index, int) or row_index < 0:
                    raise ValueError("row_index must be a non-negative integer")
                if (
                    not isinstance(row_sha256, str)
                    or len(row_sha256) != 64
                    or any(character not in "0123456789abcdef" for character in row_sha256)
                ):
                    raise ValueError("row_sha256 must be a SHA-256 hex identity")
                split_ids.append(sample_id)
                task_rows.append((row_index, row_sha256))
            if len(split_ids) != len(set(split_ids)):
                raise ValueError("duplicate IDs within a sample split are forbidden")
            task_ids.extend(split_ids)
        if len(task_ids) != len(set(task_ids)):
            raise ValueError("generation/calibration/test members must be disjoint")
        row_hashes: dict[int, str] = {}
        for row_index, row_sha256 in task_rows:
            previous_hash = row_hashes.get(row_index)
            if previous_hash is None:
                row_hashes[row_index] = row_sha256
            elif previous_hash != row_sha256:
                raise ValueError("one source row index has conflicting row_sha256 identities")
            else:
                raise ValueError(
                    "source-row identities must be unique within and across sample splits"
                )
        all_ids.extend(task_ids)
    if len(all_ids) != len(set(all_ids)):
        raise ValueError("sample IDs must be globally unique across task/split identity")


def sample_set_digest(sample_sets: Mapping[str, Any]) -> str:
    validate_sample_sets(sample_sets)
    return canonical_sha256(dict(sample_sets))


@dataclass(frozen=True)
class EnvironmentIdentity:
    python_version: str
    platform: str
    cpu: str
    ram: str
    gpu: tuple[str, ...]
    gpu_vram: tuple[str, ...]
    cuda: str | None
    driver: str | None
    torch_version: str
    transformers_version: str
    datasets_version: str


REQUIRED_MANIFEST_FIELDS = frozenset(
    {
        "schema_version",
        "experiment_id",
        "experiment_spec_id",
        "run_id",
        "artifact_set_id",
        "created_by_tool_version",
        "git_commit",
        "git_dirty",
        "git_diff_sha256",
        "environment",
        "model_repo",
        "model_revision",
        "model_artifact_hashes",
        "tokenizer_repo",
        "tokenizer_revision",
        "tokenizer_artifact_hashes",
        "sample_sets",
        "sample_set_sha256",
        "watermark_config",
        "authority_record",
        "generation_config",
        "detector_config",
        "calibration_policy",
        "evidence_length_policy",
        "threshold_records",
        "transform_config",
        "search_config",
        "rng_seeds",
        "environment_lock_sha256",
        "source_ledger_sha256",
        "checkpoint",
        "scientific_artifact_sha256",
        "experiment_specification",
    }
)


def validate_manifest(manifest: Mapping[str, Any]) -> None:
    missing = REQUIRED_MANIFEST_FIELDS - manifest.keys()
    if missing:
        raise ValueError(f"manifest missing fields: {', '.join(sorted(missing))}")
    specification = manifest["experiment_specification"]
    if not isinstance(specification, Mapping):
        raise TypeError("experiment_specification must be an object")
    expected_spec_id = experiment_specification_id(specification)
    if manifest["experiment_spec_id"] != expected_spec_id:
        raise ValueError("experiment_spec_id does not match canonical specification")
    sample_sets = manifest["sample_sets"]
    if not isinstance(sample_sets, Mapping):
        raise TypeError("sample_sets must be an object")
    validate_sample_sets(sample_sets)
    if specification.get("sample_sets") != sample_sets:
        raise ValueError("manifest and experiment specification sample sets differ")
    duplicate_bindings = {
        "model_repo": specification.get("model", {}).get("repo"),
        "model_revision": specification.get("model", {}).get("revision"),
        "model_artifact_hashes": specification.get("model", {}).get("artifact_hashes"),
        "tokenizer_repo": specification.get("tokenizer", {}).get("repo"),
        "tokenizer_revision": specification.get("tokenizer", {}).get("revision"),
        "tokenizer_artifact_hashes": specification.get("tokenizer", {}).get("artifact_hashes"),
        "watermark_config": specification.get("schemes"),
        "detector_config": specification.get("schemes"),
        "generation_config": specification.get("generation_config"),
        "calibration_policy": specification.get("calibration_policy"),
        "evidence_length_policy": specification.get("evidence_length_policy"),
        "transform_config": specification.get("transform_config"),
        "search_config": specification.get("search_config"),
        "environment_lock_sha256": specification.get("environment_lock_sha256"),
        "source_ledger_sha256": specification.get("source_authorities", {}).get(
            "source_ledger_sha256"
        ),
    }
    for field, expected in duplicate_bindings.items():
        if manifest.get(field) != expected:
            raise ValueError(f"manifest scientific field differs from specification: {field}")
    expected_sample_digest = sample_set_digest(sample_sets)
    if manifest["sample_set_sha256"] != expected_sample_digest:
        raise ValueError("sample_set_sha256 does not match exact members and rows")
    checkpoint = manifest["checkpoint"]
    if not isinstance(checkpoint, Mapping):
        raise TypeError("checkpoint must be an object")
    payload_sha = checkpoint.get("payload_sha256")
    if not isinstance(payload_sha, str):
        raise ValueError("checkpoint payload identity is missing")
    expected_run_id = execution_run_id(expected_spec_id, payload_sha)
    if manifest["run_id"] != expected_run_id:
        raise ValueError("run_id does not bind specification and checkpoint payload")
    checkpoint_expectations = {
        "experiment_spec_id": expected_spec_id,
        "sample_set_sha256": expected_sample_digest,
        "model_identity_sha256": canonical_sha256(specification.get("model")),
        "tokenizer_identity_sha256": canonical_sha256(specification.get("tokenizer")),
        "generation_config_sha256": canonical_sha256(specification.get("generation_config")),
    }
    for field, expected in checkpoint_expectations.items():
        if checkpoint.get(field) != expected:
            raise ValueError(f"manifest checkpoint identity mismatch: {field}")
    artifacts = manifest["scientific_artifact_sha256"]
    if not isinstance(artifacts, Mapping) or not artifacts:
        raise ValueError("scientific_artifact_sha256 must not be empty")
    if manifest["artifact_set_id"] != artifact_set_id(
        {str(key): str(value) for key, value in artifacts.items()}
    ):
        raise ValueError("artifact_set_id does not bind scientific artifacts")
    for name, digest in artifacts.items():
        if Path(str(name)).name != name or not isinstance(digest, str) or len(digest) != 64:
            raise ValueError("scientific artifact identities must be safe SHA-256 entries")
    checkpoint_path = checkpoint.get("path")
    if (
        not isinstance(checkpoint_path, str)
        or Path(checkpoint_path).name != checkpoint_path
        or checkpoint_path not in artifacts
        or checkpoint.get("file_sha256") != artifacts[checkpoint_path]
    ):
        raise ValueError("checkpoint path and file digest must bind a scientific artifact")
    threshold_records = manifest["threshold_records"]
    if not isinstance(threshold_records, list):
        raise TypeError("threshold_records must be an array")
    threshold_ids = [record.get("threshold_id") for record in threshold_records]
    if len(threshold_ids) != len(set(threshold_ids)):
        raise ValueError("threshold IDs must be unique")
    _validate_json(dict(manifest))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json_bytes(value, terminal_newline=True))


def write_jsonl(path: Path, records: Sequence[Mapping[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = b"".join(
        canonical_json_bytes(dict(record), terminal_newline=True) for record in records
    )
    path.write_bytes(payload)


def verify_artifact_checksums(run_dir: Path) -> dict[str, str]:
    manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
    validate_manifest(manifest)
    expected = manifest["scientific_artifact_sha256"]
    actual: dict[str, str] = {}
    for name, digest in expected.items():
        path = run_dir / name
        if not path.is_file():
            raise FileNotFoundError(f"scientific artifact is missing: {name}")
        actual[name] = sha256_hex(path.read_bytes())
        if actual[name] != digest:
            raise ValueError(f"scientific artifact checksum mismatch: {name}")
    if artifact_set_id(actual) != manifest["artifact_set_id"]:
        raise ValueError("verified artifact set identity mismatch")
    checkpoint = manifest["checkpoint"]
    from waterlarp.checkpoints import load_checkpoint

    load_checkpoint(
        run_dir / checkpoint["path"],
        {
            field: checkpoint[field]
            for field in (
                "experiment_spec_id",
                "sample_set_sha256",
                "model_identity_sha256",
                "tokenizer_identity_sha256",
                "generation_config_sha256",
            )
        },
    )
    checksums_path = run_dir / "checksums.json"
    if checksums_path.is_file():
        recorded = json.loads(checksums_path.read_text(encoding="utf-8"))
        for name, digest in recorded.items():
            path = run_dir / name
            if not path.is_file() or sha256_hex(path.read_bytes()) != digest:
                raise ValueError(f"run checksum mismatch: {name}")
    return actual


def environment_to_dict(identity: EnvironmentIdentity) -> dict[str, Any]:
    return asdict(identity)
