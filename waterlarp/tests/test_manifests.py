import copy
from pathlib import Path

import pytest
import yaml

from waterlarp.config import (
    PILOT_EXECUTION_SCOPE,
    PILOT_PAPER_PLAN_STATUS,
    PILOT_PUBLICATION_STATUS,
)
from waterlarp.manifests import (
    REQUIRED_MANIFEST_FIELDS,
    artifact_set_id,
    canonical_json_bytes,
    canonical_sha256,
    execution_run_id,
    experiment_specification_id,
    sample_set_digest,
    validate_manifest,
    validate_sample_sets,
)


def sample_sets(count: int = 2) -> dict[str, object]:
    return {
        task: {
            "dataset": f"fixture/{task}",
            "dataset_revision": "a" * 40,
            "source_split": "test",
            "prompt_template_sha256": "b" * 64,
            "splits": {
                split: [
                    {
                        "sample_id": f"{task}-{split}-{index}",
                        "row_index": offset + index,
                        "row_sha256": f"{offset + index:064x}",
                    }
                    for index in range(count)
                ]
                for offset, split in ((0, "generation"), (10000, "calibration"), (20000, "test"))
            },
        }
        for task in ("c4", "gsm8k", "mbpp")
    }


def complete_manifest() -> dict[str, object]:
    sets = sample_sets()
    specification = {
        "sample_sets": sets,
        "evaluation_lengths": [32, 64],
        "generation_config": {"temperature": 0.8},
        "model": {"repo": "fixture/model", "revision": "model-revision", "artifact_hashes": {}},
        "tokenizer": {
            "repo": "fixture/model",
            "revision": "tokenizer-revision",
            "artifact_hashes": {},
        },
        "schemes": {},
        "calibration_policy": {},
        "evidence_length_policy": {},
        "transform_config": {},
        "mixed_authorship": {},
        "search_config": {},
        "source_authorities": {"source_ledger_sha256": "9" * 64},
        "environment_lock_sha256": "a" * 64,
        "master_seed": 1729,
    }
    experiment_spec_id = experiment_specification_id(specification)
    checkpoint_sha = "c" * 64
    artifacts = {
        "generation-checkpoint.json": "d" * 64,
        "examples.jsonl": "e" * 64,
        "aggregate.json": "f" * 64,
    }
    manifest = {field: "fixture" for field in REQUIRED_MANIFEST_FIELDS}
    manifest.update(
        {
            "schema_version": "2.0.0",
            "experiment_specification": specification,
            "experiment_spec_id": experiment_spec_id,
            "run_id": execution_run_id(experiment_spec_id, checkpoint_sha),
            "artifact_set_id": artifact_set_id(artifacts),
            "git_commit": "0" * 40,
            "git_dirty": True,
            "environment": {},
            "model_repo": "fixture/model",
            "model_revision": "model-revision",
            "model_artifact_hashes": {},
            "tokenizer_repo": "fixture/model",
            "tokenizer_revision": "tokenizer-revision",
            "tokenizer_artifact_hashes": {},
            "sample_sets": sets,
            "sample_set_sha256": sample_set_digest(sets),
            "watermark_config": {},
            "authority_record": {},
            "generation_config": specification["generation_config"],
            "detector_config": {},
            "calibration_policy": {},
            "evidence_length_policy": {},
            "threshold_records": [],
            "transform_config": {},
            "search_config": {},
            "rng_seeds": {},
            "environment_lock_sha256": "a" * 64,
            "source_ledger_sha256": "9" * 64,
            "checkpoint": {
                "path": "generation-checkpoint.json",
                "payload_sha256": checkpoint_sha,
                "file_sha256": "d" * 64,
                "experiment_spec_id": experiment_spec_id,
                "sample_set_sha256": sample_set_digest(sets),
                "model_identity_sha256": canonical_sha256(specification["model"]),
                "tokenizer_identity_sha256": canonical_sha256(specification["tokenizer"]),
                "generation_config_sha256": canonical_sha256(specification["generation_config"]),
            },
            "scientific_artifact_sha256": artifacts,
        }
    )
    return manifest


def test_canonical_json_and_spec_id_are_order_independent() -> None:
    left = {"é": [1, True], "a": 2}
    right = {"a": 2, "é": [1, True]}
    assert canonical_json_bytes(left) == b'{"a":2,"\xc3\xa9":[1,true]}'
    assert experiment_specification_id(left) == experiment_specification_id(right)


def test_nonfinite_json_is_rejected() -> None:
    with pytest.raises(ValueError, match="non-finite"):
        canonical_json_bytes({"bad": float("nan")})


def test_manifest_binds_specification_checkpoint_and_artifacts() -> None:
    manifest = complete_manifest()
    validate_manifest(manifest)
    changed = copy.deepcopy(manifest)
    changed["experiment_specification"]["generation_config"]["temperature"] = 0.9  # type: ignore[index]
    with pytest.raises(ValueError, match="experiment_spec_id"):
        validate_manifest(changed)
    changed = copy.deepcopy(manifest)
    changed["checkpoint"]["payload_sha256"] = "1" * 64  # type: ignore[index]
    with pytest.raises(ValueError, match="run_id"):
        validate_manifest(changed)
    changed = copy.deepcopy(manifest)
    changed["scientific_artifact_sha256"]["examples.jsonl"] = "2" * 64  # type: ignore[index]
    with pytest.raises(ValueError, match="artifact_set_id"):
        validate_manifest(changed)


def test_evaluation_lengths_cached_rows_and_model_identity_change_specification() -> None:
    manifest = complete_manifest()
    original = manifest["experiment_spec_id"]
    for mutate in (
        lambda specification: specification["evaluation_lengths"].__setitem__(0, 16),
        lambda specification: specification["sample_sets"]["c4"]["splits"]["test"][0].__setitem__(
            "row_sha256", "8" * 64
        ),
        lambda specification: specification["model"].__setitem__("revision", "changed-model"),
        lambda specification: specification["tokenizer"].__setitem__(
            "revision", "changed-tokenizer"
        ),
    ):
        changed = copy.deepcopy(manifest["experiment_specification"])
        mutate(changed)
        assert experiment_specification_id(changed) != original


def test_arbitrary_n_members_round_trip_and_select_identity() -> None:
    sets = sample_sets(4)
    encoded = canonical_json_bytes(sets)
    assert encoded == canonical_json_bytes(copy.deepcopy(sets))
    validate_sample_sets(sets)
    changed = copy.deepcopy(sets)
    changed["c4"]["splits"]["test"][2]["sample_id"] = "c4-test-changed"  # type: ignore[index]
    assert sample_set_digest(sets) != sample_set_digest(changed)
    assert experiment_specification_id({"sample_sets": sets}) != experiment_specification_id(
        {"sample_sets": changed}
    )


def test_sample_set_duplicates_and_cross_split_leakage_are_rejected() -> None:
    within = sample_sets()
    within["c4"]["splits"]["test"][1] = copy.deepcopy(  # type: ignore[index]
        within["c4"]["splits"]["test"][0]  # type: ignore[index]
    )
    with pytest.raises(ValueError, match="duplicate IDs within"):
        validate_sample_sets(within)
    crossing = sample_sets()
    crossing["c4"]["splits"]["test"][0]["sample_id"] = (  # type: ignore[index]
        crossing["c4"]["splits"]["calibration"][0]["sample_id"]  # type: ignore[index]
    )
    with pytest.raises(ValueError, match="disjoint"):
        validate_sample_sets(crossing)


def test_source_row_alias_within_split_is_rejected() -> None:
    sets = sample_sets()
    first = sets["c4"]["splits"]["test"][0]  # type: ignore[index]
    alias = sets["c4"]["splits"]["test"][1]  # type: ignore[index]
    alias["row_index"] = first["row_index"]
    alias["row_sha256"] = first["row_sha256"]

    with pytest.raises(ValueError, match="source-row identities must be unique"):
        validate_sample_sets(sets)


def test_source_row_alias_across_calibration_and_test_is_rejected() -> None:
    sets = sample_sets()
    calibration = sets["c4"]["splits"]["calibration"][0]  # type: ignore[index]
    test = sets["c4"]["splits"]["test"][0]  # type: ignore[index]
    test["row_index"] = calibration["row_index"]
    test["row_sha256"] = calibration["row_sha256"]

    with pytest.raises(ValueError, match="source-row identities must be unique"):
        validate_sample_sets(sets)


def test_source_row_index_with_conflicting_hashes_is_rejected() -> None:
    sets = sample_sets()
    calibration = sets["c4"]["splits"]["calibration"][0]  # type: ignore[index]
    test = sets["c4"]["splits"]["test"][0]  # type: ignore[index]
    test["row_index"] = calibration["row_index"]
    assert test["row_sha256"] != calibration["row_sha256"]

    with pytest.raises(ValueError, match="conflicting row_sha256"):
        validate_sample_sets(sets)


def test_paper_plan_sized_synthetic_member_sets_validate() -> None:
    sets = sample_sets(1000)
    validate_sample_sets(sets)
    assert (
        sum(
            len(members)
            for task in sets.values()
            for members in task["splits"].values()  # type: ignore[union-attr]
        )
        == 9000
    )


def test_cpu_pilot_remains_integration_only_and_excludes_provider_authorities() -> None:
    package_root = Path(__file__).resolve().parents[1]
    profile = yaml.safe_load((package_root / "configs/pilot/cpu.yaml").read_text(encoding="utf-8"))
    assert PILOT_EXECUTION_SCOPE == "INTEGRATION_PILOT"
    assert PILOT_PUBLICATION_STATUS == "PILOT_NOT_BENCHMARK_EVIDENCE"
    assert PILOT_PAPER_PLAN_STATUS == "NOT_EXECUTED"
    assert profile["label"] == "PILOT"
    assert profile["publication_status"] == "PILOT_NOT_HEADLINE"
    assert profile["schemes"] == ["reference.kgw", "reference.synthid_text"]
    assert "anthropic.embedded_text_watermark" not in profile["schemes"]
