import copy
import json
from pathlib import Path

import pytest

from waterlarp.checkpoints import (
    append_entry,
    load_checkpoint,
    new_checkpoint,
    write_checkpoint,
)
from waterlarp.manifests import canonical_json_bytes, execution_run_id


def identity() -> dict[str, str]:
    return {
        "experiment_spec_id": "wlrs1-" + "1" * 24,
        "sample_set_sha256": "2" * 64,
        "model_identity_sha256": "3" * 64,
        "tokenizer_identity_sha256": "4" * 64,
        "generation_config_sha256": "5" * 64,
    }


def entry(tokens: list[int]) -> dict[str, object]:
    return {
        "task": "c4",
        "split": "test",
        "sample_id": "c4-test-1",
        "kind": "kgw",
        "prompt_token_ids": [1, 2],
        "generated_token_ids": tokens,
        "steps": [],
    }


def test_checkpoint_round_trip_and_tamper_rejection(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    checkpoint = new_checkpoint(identity())
    write_checkpoint(path, checkpoint)
    append_entry(path, checkpoint, entry([7, 8, 9]))
    loaded = load_checkpoint(path, identity())
    assert loaded["entries"][0]["generated_token_ids"] == [7, 8, 9]
    raw = bytearray(path.read_bytes())
    raw[10] ^= 1
    path.write_bytes(raw)
    with pytest.raises(ValueError, match="canonical JSON|canonical"):
        load_checkpoint(path, identity())


def test_stale_spec_model_and_tokenizer_identities_are_rejected(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    checkpoint = new_checkpoint(identity())
    write_checkpoint(path, checkpoint)
    for field in (
        "experiment_spec_id",
        "sample_set_sha256",
        "model_identity_sha256",
        "tokenizer_identity_sha256",
        "generation_config_sha256",
    ):
        changed = identity()
        changed[field] = "9" * len(changed[field])
        with pytest.raises(ValueError, match=field):
            load_checkpoint(path, changed)


def test_generated_sequence_mutation_rejects_or_changes_run_id(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    checkpoint = new_checkpoint(identity())
    append_entry(path, checkpoint, entry([1, 2, 3]))
    original_payload = checkpoint["payload_sha256"]
    document = json.loads(path.read_text(encoding="utf-8"))
    document["entries"][0]["generated_token_ids"][1] = 99
    path.write_bytes(canonical_json_bytes(document, terminal_newline=True))
    with pytest.raises(ValueError, match="payload checksum"):
        load_checkpoint(path, identity())
    changed = copy.deepcopy(checkpoint)
    changed["entries"][0]["generated_token_ids"][1] = 99
    write_checkpoint(path, changed)
    assert changed["payload_sha256"] != original_payload
    assert execution_run_id(identity()["experiment_spec_id"], changed["payload_sha256"]) != (
        execution_run_id(identity()["experiment_spec_id"], original_payload)
    )


def test_checkpoint_replacement_leaves_no_partial_file(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    checkpoint = new_checkpoint(identity())
    write_checkpoint(path, checkpoint)
    append_entry(path, checkpoint, entry([1, 2, 3]))
    loaded = load_checkpoint(path, identity())
    assert loaded["entries"][0]["generated_token_ids"] == [1, 2, 3]
    assert not path.with_suffix(".json.tmp").exists()
