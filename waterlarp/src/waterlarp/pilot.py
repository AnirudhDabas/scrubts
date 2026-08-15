"""Small real CPU PILOT; integration evidence only, never benchmark evidence."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from collections import defaultdict
from collections.abc import Callable, Mapping, Sequence
from dataclasses import asdict, dataclass, replace
from pathlib import Path
from typing import Any, cast

import numpy as np
import torch
import yaml
from transformers import AutoModelForCausalLM, AutoTokenizer

from waterlarp import __version__
from waterlarp.adapters.base import DetectionScore
from waterlarp.adapters.kgw import KGW_REFERENCE_COMMIT, KgwAdapter, KgwConfig
from waterlarp.adapters.synthid import (
    SYNTHID_REFERENCE_COMMIT,
    TRANSFORMERS_REFERENCE_COMMIT,
    SynthIdAdapter,
    SynthIdConfig,
    synthid_keys,
)
from waterlarp.authority import KGW_AUTHORITY, SYNTHID_AUTHORITY, AuthorityRecord
from waterlarp.calibration.thresholds import (
    ThresholdCalibration,
    ThresholdContext,
    calibrate_threshold,
    select_threshold,
    selected_decision,
)
from waterlarp.checkpoints import (
    append_entry,
    load_checkpoint,
    new_checkpoint,
    validate_checkpoint,
    write_checkpoint,
)
from waterlarp.composition.spans import compose_tokens
from waterlarp.composition.windows import WindowSearchSpecification, fixed_window_max
from waterlarp.config import (
    PILOT_EXECUTION_SCOPE,
    PILOT_PAPER_PLAN_STATUS,
    PILOT_PUBLICATION_STATUS,
    Comparator,
    CoordinateSystem,
    ThresholdSemantics,
)
from waterlarp.datasets.base import DatasetIdentity, DatasetSample, fixed_count_splits
from waterlarp.datasets.c4 import IDENTITY as C4_IDENTITY
from waterlarp.datasets.c4 import adapt as adapt_c4
from waterlarp.datasets.gsm8k import IDENTITY as GSM8K_IDENTITY
from waterlarp.datasets.gsm8k import adapt as adapt_gsm8k
from waterlarp.datasets.mbpp import IDENTITY as MBPP_IDENTITY
from waterlarp.datasets.mbpp import adapt as adapt_mbpp
from waterlarp.entropy.metrics import summarize
from waterlarp.evidence import canonical_detector_evidence, detector_config_sha256
from waterlarp.generation.runner import GeneratedExample, generate_autoregressive
from waterlarp.manifests import (
    artifact_set_id,
    canonical_sha256,
    execution_run_id,
    experiment_specification_id,
    sample_set_digest,
    validate_manifest,
    verify_artifact_checksums,
    write_json,
    write_jsonl,
)
from waterlarp.metrics.aggregate import aggregate_records
from waterlarp.metrics.localization import localization_report
from waterlarp.quality.general import normalized_token_edit_distance
from waterlarp.quality.gsm8k import answer_preserved
from waterlarp.quality.integrity import literal_integrity
from waterlarp.rng import benchmark_key, derive_seed
from waterlarp.transforms.token_edits import random_deletion


@dataclass(frozen=True)
class CachedMember:
    sample: DatasetSample
    row_index: int
    row_sha256: str


@dataclass(frozen=True)
class CachedTask:
    identity: DatasetIdentity
    members: tuple[CachedMember, ...]


@dataclass(frozen=True)
class SchemeRuntime:
    name: str
    short_name: str
    adapter: Any
    authority: AuthorityRecord
    comparator: Comparator
    comparator_authority: str


def _sha(value: str | bytes) -> str:
    return hashlib.sha256(value.encode() if isinstance(value, str) else value).hexdigest()


def _git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()


def _checkout_head(checkout: Path) -> str:
    safe = checkout.resolve().as_posix()
    return subprocess.run(
        ["git", "-c", f"safe.directory={safe}", "-C", str(checkout), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _verify_checkout(checkout: Path, expected_head: str) -> None:
    if _checkout_head(checkout) != expected_head:
        raise ValueError("KGW checkout is not the pinned authority revision")
    safe = checkout.resolve().as_posix()
    status = subprocess.run(
        [
            "git",
            "-c",
            f"safe.directory={safe}",
            "-C",
            str(checkout),
            "status",
            "--porcelain",
            "--untracked-files=all",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    material = [
        line
        for line in status
        if not (
            line.startswith("?? ")
            and "/__pycache__/" in line.replace("\\", "/")
            and line.endswith(".pyc")
        )
    ]
    if material:
        raise ValueError("KGW authority checkout has material working-tree changes")


def _diff_hash(root: Path) -> str:
    tracked_bytes = subprocess.run(
        ["git", "diff", "--binary", "HEAD"], cwd=root, check=True, capture_output=True
    ).stdout
    untracked = _git(root, "ls-files", "--others", "--exclude-standard").splitlines()
    identities = []
    for relative in sorted(untracked):
        path = root / relative
        identities.append({"path": relative.replace("\\", "/"), "sha256": _sha(path.read_bytes())})
    return canonical_sha256(
        {
            "tracked_diff_utf8": tracked_bytes.decode("utf-8", errors="replace"),
            "untracked": identities,
        }
    )


def _artifact_hashes(snapshot: Path, names: Sequence[str]) -> dict[str, str]:
    return {
        name: _sha((snapshot / name).read_bytes()) for name in names if (snapshot / name).is_file()
    }


def _load_cached_task(
    cache: Path,
    name: str,
    identity: DatasetIdentity,
    adapter: Callable[[dict[str, object], int], DatasetSample],
) -> CachedTask:
    document = json.loads((cache / "pilot-rows" / f"{name}.json").read_text(encoding="utf-8"))
    members: list[CachedMember] = []
    for item in document["rows"]:
        row_index = int(item["row_idx"])
        row = cast(dict[str, object], item["row"])
        members.append(CachedMember(adapter(row, row_index), row_index, canonical_sha256(item)))
    return CachedTask(identity, tuple(members))


def _cached_tasks(cache: Path) -> dict[str, CachedTask]:
    return {
        "c4": _load_cached_task(cache, "c4", C4_IDENTITY, adapt_c4),
        "gsm8k": _load_cached_task(cache, "gsm8k", GSM8K_IDENTITY, adapt_gsm8k),
        "mbpp": _load_cached_task(cache, "mbpp", MBPP_IDENTITY, adapt_mbpp),
    }


def _select_members(
    tasks: Mapping[str, CachedTask], config: Mapping[str, Any], master_seed: int
) -> tuple[dict[str, dict[str, tuple[CachedMember, ...]]], dict[str, Any]]:
    configured = {str(item["dataset"]): item for item in config["tasks"]}
    selected: dict[str, dict[str, tuple[CachedMember, ...]]] = {}
    sample_sets: dict[str, Any] = {}
    for task, cached in sorted(tasks.items()):
        task_config = configured[cached.identity.repo]
        if task_config["revision"] != cached.identity.revision:
            raise ValueError(f"configured dataset revision differs from adapter identity: {task}")
        count = int(task_config["sample_count_per_split"])
        splits = fixed_count_splits(
            (member.sample.sample_id for member in cached.members),
            per_split=count,
            seed=master_seed,
        )
        by_id = {member.sample.sample_id: member for member in cached.members}
        selected[task] = {
            split: tuple(by_id[sample_id] for sample_id in ids) for split, ids in splits.items()
        }
        sample_sets[task] = {
            "dataset": cached.identity.repo,
            "dataset_revision": cached.identity.revision,
            "source_split": cached.identity.split,
            "prompt_template_sha256": cached.identity.prompt_template_sha256,
            "splits": {
                split: [
                    {
                        "sample_id": member.sample.sample_id,
                        "row_index": member.row_index,
                        "row_sha256": member.row_sha256,
                    }
                    for member in selected[task][split]
                ]
                for split in ("generation", "calibration", "test")
            },
        }
    sample_set_digest(sample_sets)
    return selected, sample_sets


def _checkpoint_entry(example: GeneratedExample, **identity: str) -> dict[str, Any]:
    return {
        **identity,
        "prompt_token_ids": list(example.prompt_token_ids),
        "generated_token_ids": list(example.generated_token_ids),
        "steps": [asdict(step) for step in example.steps],
    }


def _entry_map(checkpoint: Mapping[str, Any]) -> dict[tuple[str, str, str, str], dict[str, Any]]:
    return {
        (entry["task"], entry["split"], entry["sample_id"], entry["kind"]): entry
        for entry in checkpoint["entries"]
    }


def _tokens(entry: Mapping[str, Any], length: int | None = None) -> list[int]:
    values = [int(value) for value in entry["generated_token_ids"]]
    return values if length is None else values[:length]


def _entropy(entry: Mapping[str, Any], length: int) -> dict[str, Any]:
    steps = entry["steps"][:length]
    shannon = summarize(np.asarray([step["shannon_entropy"] for step in steps]))
    spike = summarize(np.asarray([step["spike_entropy"] for step in steps]))
    return {
        "scope": "immutable base-model next-token logits before all processors",
        "shannon": asdict(shannon),
        "spike_entrobench": asdict(spike),
        "token_count": len(steps),
        "base_logits_sha256": [step["base_logits_sha256"] for step in steps],
    }


def _decode(tokenizer: Any, token_ids: Sequence[int]) -> str:
    decoded = tokenizer.decode(token_ids, skip_special_tokens=True)
    if not isinstance(decoded, str):
        raise TypeError("single token sequence decode did not return text")
    return decoded


def _context(
    runtime: SchemeRuntime,
    *,
    model_revision: str,
    tokenizer_revision: str,
    task: str,
    scored_unit_count: int,
    semantics: ThresholdSemantics,
    regime: str,
    operation: str | None = None,
    operation_strength: float | None = None,
    search_spec_sha256: str | None = None,
) -> ThresholdContext:
    return ThresholdContext(
        scheme=runtime.name,
        mechanism_version=runtime.authority.mechanism_version,
        detector_config_sha256=detector_config_sha256(runtime.adapter.metadata()),
        model_revision=model_revision,
        tokenizer_revision=tokenizer_revision,
        task=task,
        key_policy=runtime.authority.key_provenance,
        threshold_semantics=semantics,
        evidence_length_policy="exact_observed_detector_scored_units_v1",
        scored_unit_count=scored_unit_count,
        regime=regime,
        operation=operation,
        operation_strength=operation_strength,
        search_spec_sha256=search_spec_sha256,
        comparator=runtime.comparator,
        comparator_authority=runtime.comparator_authority,
    )


def _calibrate_groups(
    groups: Mapping[int, list[tuple[str, float]]],
    base_context: ThresholdContext,
    target_fpr: float,
    threshold_records: list[ThresholdCalibration],
    provenance: dict[str, dict[str, Any]],
) -> None:
    for evidence_length, values in sorted(groups.items()):
        context = replace(base_context, scored_unit_count=evidence_length)
        calibration = calibrate_threshold(
            (score for _, score in values),
            target_fpr,
            context=context,
            comparator=context.comparator,
        )
        threshold_records.append(calibration)
        provenance[calibration.threshold_id] = {
            "calibration_sample_ids": [sample_id for sample_id, _ in values],
            "calibration_scores_sha256": canonical_sha256([score for _, score in values]),
        }


def _score_groups(
    runtime: SchemeRuntime,
    members: Sequence[CachedMember],
    entries: Mapping[tuple[str, str, str, str], Mapping[str, Any]],
    tokenizer: Any,
    *,
    task: str,
    split: str,
    kind: str,
    length: int,
    transform: Callable[[str, list[int]], list[int]] | None = None,
) -> dict[int, list[tuple[str, float]]]:
    groups: dict[int, list[tuple[str, float]]] = defaultdict(list)
    for member in members:
        source = _tokens(entries[(task, split, member.sample.sample_id, kind)], length)
        token_ids = source if transform is None else transform(member.sample.sample_id, source)
        score = runtime.adapter.score_token_ids(token_ids, tokenizer)
        groups[score.scored_unit_count].append((member.sample.sample_id, score.score))
    return groups


def _search_score(
    runtime: SchemeRuntime,
    tokenizer: Any,
    token_ids: Sequence[int],
    specification: WindowSearchSpecification,
) -> tuple[DetectionScore, dict[str, int | float]]:
    def scorer(values: Sequence[int]) -> float:
        return float(runtime.adapter.score_token_ids(list(values), tokenizer).score)

    searched = fixed_window_max(token_ids, scorer, specification.window_size, specification.stride)
    best_tokens = list(token_ids[searched.maximum.start : searched.maximum.end])
    score = runtime.adapter.score_token_ids(best_tokens, tokenizer)
    return score, asdict(searched.maximum)


def _make_record(
    *,
    run_id: str,
    runtime: SchemeRuntime,
    tokenizer: Any,
    tokenizer_identity: Mapping[str, Any],
    thresholds: Sequence[ThresholdCalibration],
    model_revision: str,
    tokenizer_revision: str,
    task: str,
    sample_id: str,
    label: str,
    token_ids: Sequence[int],
    nominal_generation_length: int,
    entropy: Mapping[str, Any] | None,
    semantics: ThresholdSemantics,
    regime: str,
    evaluation_kind: str,
    transform: str,
    transform_strength: float,
    quality: Mapping[str, Any],
    input_sha256: str,
    marked_fraction: float,
    layout: str | None = None,
    threat_model: Mapping[str, Any] | None = None,
    operation: str | None = None,
    operation_strength: float | None = None,
    search_specification: WindowSearchSpecification | None = None,
    localization: Mapping[str, Any] | None = None,
    segments: Sequence[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    procedure: dict[str, Any] | None = None
    if search_specification is None:
        score = runtime.adapter.score_token_ids(list(token_ids), tokenizer)
        search_sha = None
        search_spec_id = None
    else:
        score, best_window = _search_score(runtime, tokenizer, token_ids, search_specification)
        search_sha = search_specification.sha256
        search_spec_id = search_specification.search_spec_id
        procedure = {
            **asdict(search_specification),
            "search_spec_id": search_spec_id,
            "search_spec_sha256": search_sha,
            "best_window": best_window,
        }
    request = _context(
        runtime,
        model_revision=model_revision,
        tokenizer_revision=tokenizer_revision,
        task=task,
        scored_unit_count=score.scored_unit_count,
        semantics=semantics,
        regime=regime,
        operation=operation,
        operation_strength=operation_strength,
        search_spec_sha256=search_sha,
    )
    selection = select_threshold(thresholds, request)
    decision, decision_status = selected_decision(score.score, selection)
    detector_evidence = canonical_detector_evidence(
        detector_input_token_ids=token_ids,
        tokenizer_identity=tokenizer_identity,
        detector_metadata=runtime.adapter.metadata(),
        key_provenance=runtime.authority.key_provenance,
        score=score,
        threshold_request=request,
        selection=selection,
        decision=decision,
        decision_status=decision_status,
        procedure=procedure,
    )
    threshold_id = None if selection.threshold is None else selection.threshold.threshold_id
    output_sha = canonical_sha256([int(value) for value in token_ids])
    identity = {
        "run_id": run_id,
        "sample_id": sample_id,
        "scheme": runtime.name,
        "evaluation_kind": evaluation_kind,
        "transform": transform,
        "nominal_generation_length": nominal_generation_length,
        "label": label,
        "output_sha256": output_sha,
        "threshold_id": threshold_id,
        "layout": layout,
        "marked_fraction": marked_fraction,
    }
    return {
        "schema_version": "2.0.0",
        "evaluation_id": f"wlre1-{canonical_sha256(identity)[:24]}",
        "run_id": run_id,
        "manifest_path": "manifest.json",
        "source_authority_ids": list(runtime.authority.authority_source_ids),
        "sample_id": sample_id,
        "source_split": "test",
        "task": task,
        "scheme": runtime.name,
        "mode": "authoritative_default",
        "label": label,
        "evaluation_kind": evaluation_kind,
        "nominal_generation_length": nominal_generation_length,
        "observed_token_length": len(token_ids),
        "evidence_length": score.scored_unit_count,
        "entropy": None if entropy is None else dict(entropy),
        "entropy_bucket": {"c4": "high", "gsm8k": "medium", "mbpp": "low"}[task],
        "threshold_semantics": semantics,
        "threshold_id": threshold_id,
        "threshold": None if selection.threshold is None else selection.threshold.threshold,
        "comparator": None if selection.threshold is None else selection.threshold.comparator,
        "score": score.score,
        "p_value": score.p_value,
        "decision": decision,
        "decision_status": decision_status,
        "threshold_selection_status": selection.status,
        "threshold_selection_reason": selection.reason,
        "transform": transform,
        "transform_strength": transform_strength,
        "marked_fraction": marked_fraction,
        "layout": layout,
        "search_spec_id": search_spec_id,
        "threat_model": None if threat_model is None else dict(threat_model),
        "quality": dict(quality),
        "input_sha256": input_sha256,
        "output_sha256": output_sha,
        "detector_evidence": detector_evidence,
        "localization": None if localization is None else dict(localization),
        "segments": None if segments is None else list(segments),
    }


def run_pilot(config_path: Path, kgw_checkout: Path, output_root: Path) -> Path:
    root = config_path.resolve().parents[3]
    package_root = config_path.resolve().parents[2]
    config = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    if config["label"] != "PILOT":
        raise ValueError("pilot runner only accepts a PILOT configuration")
    _verify_checkout(kgw_checkout, KGW_REFERENCE_COMMIT)
    cache = package_root / ".cache"
    model_config = config["model"]
    os.environ.update(
        {"HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1", "HF_HOME": str(cache / "huggingface")}
    )
    snapshot = (
        cache
        / "huggingface"
        / "hub"
        / "models--HuggingFaceTB--SmolLM2-135M"
        / "snapshots"
        / model_config["revision"]
    )
    if not snapshot.is_dir():
        raise FileNotFoundError("pinned pilot model is not resolved in the ignored cache")
    master_seed = int(config["master_seed"])
    tasks = _cached_tasks(cache)
    selected, sample_sets = _select_members(tasks, config, master_seed)
    kgw = KgwAdapter(KgwConfig(base_key=benchmark_key(master_seed, "kgw")), kgw_checkout)
    synthid = SynthIdAdapter(SynthIdConfig(keys=synthid_keys(master_seed)))
    runtimes = (
        SchemeRuntime(
            "reference.kgw",
            "kgw",
            kgw,
            KGW_AUTHORITY,
            Comparator.STRICT_GREATER,
            "KGW reference detector uses z_score > z_threshold",
        ),
        SchemeRuntime(
            "reference.synthid_text",
            "synthid",
            synthid,
            SYNTHID_AUTHORITY,
            Comparator.GREATER_OR_EQUAL,
            "WaterLARP benchmark semantics; DeepMind Weighted Mean defines no tie decision",
        ),
    )
    model_artifacts = _artifact_hashes(
        snapshot,
        ("config.json", "generation_config.json", "model.safetensors", "pytorch_model.bin"),
    )
    tokenizer_artifacts = _artifact_hashes(
        snapshot,
        (
            "tokenizer.json",
            "tokenizer_config.json",
            "special_tokens_map.json",
            "vocab.json",
            "merges.txt",
        ),
    )
    lock_hash = _sha((package_root / "requirements-lock.txt").read_bytes())
    source_ledger_hash = _sha((root / "research" / "sources.yaml").read_bytes())
    evaluation_lengths = tuple(sorted(int(value) for value in config["lengths"]))
    if not evaluation_lengths or evaluation_lengths[0] <= 0:
        raise ValueError("pilot evaluation lengths must be positive")
    document_length = evaluation_lengths[-1]
    search_specification = WindowSearchSpecification(
        "TOKEN",
        document_length,
        int(config["window_search"]["window_size"]),
        int(config["window_search"]["stride"]),
        "all complete half-open windows",
        "full max procedure; exact best-window detector scored units",
    )
    search_specification.validate()
    generation_config = {
        **config["generation"],
        "max_new_tokens": document_length,
        "sampling_implementation": "waterlarp_autoregressive_v2",
        "rng": "explicit torch.Generator(cpu)",
    }
    experiment_specification = {
        "schema_version": "2.0.0",
        "profile": PILOT_EXECUTION_SCOPE,
        "code_identity": {
            "git_commit": _git(root, "rev-parse", "HEAD"),
            "git_diff_sha256": _diff_hash(root),
        },
        "model": {
            **model_config,
            "artifact_hashes": model_artifacts,
        },
        "tokenizer": {
            "repo": model_config["repo"],
            "revision": model_config["tokenizer_revision"],
            "artifact_hashes": tokenizer_artifacts,
        },
        "sample_sets": sample_sets,
        "evaluation_lengths": list(evaluation_lengths),
        "generation_config": generation_config,
        "schemes": {runtime.name: runtime.adapter.metadata() for runtime in runtimes},
        "calibration_policy": {
            "target_fpr": float(config["target_fprs"][0]),
            "task_pooling": "FORBIDDEN",
            "comparator_is_detector_contract": True,
            "calibration_split_only": True,
            "held_out_test_split_only": True,
        },
        "evidence_length_policy": {
            "name": "exact_observed_detector_scored_units_v1",
            "selection": "EXACT_ONLY_NO_INTERPOLATION",
            "below_minimum": "UNSUPPORTED",
            "between_or_above_support": "UNRESOLVED",
            "original_nominal_length_available_to_selector": False,
        },
        "transform_config": config["transforms"],
        "mixed_authorship": config["mixed_authorship"],
        "search_config": asdict(search_specification),
        "source_authorities": {
            "kgw": KGW_REFERENCE_COMMIT,
            "synthid": SYNTHID_REFERENCE_COMMIT,
            "transformers": TRANSFORMERS_REFERENCE_COMMIT,
            "source_ledger_sha256": source_ledger_hash,
        },
        "environment_lock_sha256": lock_hash,
        "master_seed": master_seed,
    }
    experiment_spec_id = experiment_specification_id(experiment_specification)
    checkpoint_identity = {
        "experiment_spec_id": experiment_spec_id,
        "sample_set_sha256": sample_set_digest(sample_sets),
        "model_identity_sha256": canonical_sha256(experiment_specification["model"]),
        "tokenizer_identity_sha256": canonical_sha256(experiment_specification["tokenizer"]),
        "generation_config_sha256": canonical_sha256(generation_config),
    }
    checkpoint_path = output_root / "checkpoints" / f"{experiment_spec_id}.json"
    if checkpoint_path.is_file():
        checkpoint = load_checkpoint(checkpoint_path, checkpoint_identity)
    else:
        checkpoint = new_checkpoint(checkpoint_identity)
        write_checkpoint(checkpoint_path, checkpoint)
    tokenizer = cast(Any, AutoTokenizer.from_pretrained(snapshot, local_files_only=True))
    model = cast(Any, AutoModelForCausalLM.from_pretrained(snapshot, local_files_only=True))
    model.eval()
    generation_started = time.perf_counter()
    entries = _entry_map(checkpoint)
    max_new_tokens = int(generation_config["max_new_tokens"])
    for task, splits in sorted(selected.items()):
        for split_name, members in sorted(splits.items()):
            for member in members:
                for kind, scheme_runtime in (
                    ("unwatermarked", None),
                    ("kgw", runtimes[0]),
                    ("synthid", runtimes[1]),
                ):
                    key = (task, split_name, member.sample.sample_id, kind)
                    if key in entries:
                        continue
                    processors = (
                        ()
                        if scheme_runtime is None
                        else (scheme_runtime.adapter.prepare_generation(model, tokenizer, "cpu"),)
                    )
                    seed = derive_seed(
                        master_seed, f"pilot/{task}/{split_name}/{member.sample.sample_id}/base"
                    )
                    example = generate_autoregressive(
                        model=model,
                        tokenizer=tokenizer,
                        prompt=member.sample.prompt,
                        seed=seed,
                        max_new_tokens=max_new_tokens,
                        processors=processors,
                        temperature=float(generation_config["temperature"]),
                        top_k=int(generation_config["top_k"]),
                        top_p=float(generation_config["top_p"]),
                    )
                    entry = _checkpoint_entry(
                        example,
                        task=task,
                        split=split_name,
                        sample_id=member.sample.sample_id,
                        kind=kind,
                    )
                    append_entry(checkpoint_path, checkpoint, entry)
                    entries[key] = entry
    validate_checkpoint(checkpoint, checkpoint_identity)
    run_id = execution_run_id(experiment_spec_id, checkpoint["payload_sha256"])
    run_dir = output_root / run_id
    if run_dir.is_dir():
        if (run_dir / "manifest.json").is_file():
            verify_artifact_checksums(run_dir)
            return run_dir
        allowed_partial = {
            "generation-checkpoint.json",
            "examples.jsonl",
            "aggregate.json",
        }
        unexpected = {path.name for path in run_dir.iterdir()} - allowed_partial
        if unexpected:
            raise ValueError(
                "incomplete finalization directory has unexpected artifacts: "
                + ", ".join(sorted(unexpected))
            )
    else:
        run_dir.mkdir(parents=True)
    thresholds: list[ThresholdCalibration] = []
    threshold_provenance: dict[str, dict[str, Any]] = {}
    target_fpr = float(config["target_fprs"][0])
    model_revision = str(model_config["revision"])
    tokenizer_revision = str(model_config["tokenizer_revision"])
    deletion_strength = 0.1
    for task in sorted(selected):
        calibration_members = selected[task]["calibration"]
        for runtime in runtimes:
            for length_value in config["lengths"]:
                length = int(length_value)
                clean_groups = _score_groups(
                    runtime,
                    calibration_members,
                    entries,
                    tokenizer,
                    task=task,
                    split="calibration",
                    kind="unwatermarked",
                    length=length,
                )
                clean_base = _context(
                    runtime,
                    model_revision=model_revision,
                    tokenizer_revision=tokenizer_revision,
                    task=task,
                    scored_unit_count=1,
                    semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
                    regime="clean",
                )
                _calibrate_groups(
                    clean_groups, clean_base, target_fpr, thresholds, threshold_provenance
                )

                def transformed(
                    sample_id: str,
                    values: list[int],
                    task_name: str = task,
                    runtime_short: str = runtime.short_name,
                    observed_length: int = length,
                ) -> list[int]:
                    seed = derive_seed(
                        master_seed,
                        f"pilot/{task_name}/{runtime_short}/{observed_length}/"
                        f"deletion/0.1/calibration/{sample_id}",
                    )
                    return list(random_deletion(values, deletion_strength, seed).token_ids)

                operation_groups = _score_groups(
                    runtime,
                    calibration_members,
                    entries,
                    tokenizer,
                    task=task,
                    split="calibration",
                    kind="unwatermarked",
                    length=length,
                    transform=transformed,
                )
                operation_base = _context(
                    runtime,
                    model_revision=model_revision,
                    tokenizer_revision=tokenizer_revision,
                    task=task,
                    scored_unit_count=1,
                    semantics=ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD,
                    regime="operation_conditioned",
                    operation="random_deletion",
                    operation_strength=deletion_strength,
                )
                _calibrate_groups(
                    operation_groups,
                    operation_base,
                    target_fpr,
                    thresholds,
                    threshold_provenance,
                )
    for task in sorted(selected):
        for runtime in runtimes:
            search_groups: dict[int, list[tuple[str, float]]] = defaultdict(list)
            for member in selected[task]["calibration"]:
                document = _tokens(
                    entries[(task, "calibration", member.sample.sample_id, "unwatermarked")],
                    document_length,
                )
                score, _ = _search_score(runtime, tokenizer, document, search_specification)
                search_groups[score.scored_unit_count].append(
                    (member.sample.sample_id, score.score)
                )
            search_base = _context(
                runtime,
                model_revision=model_revision,
                tokenizer_revision=tokenizer_revision,
                task=task,
                scored_unit_count=1,
                semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
                regime="window_search",
                search_spec_sha256=search_specification.sha256,
            )
            _calibrate_groups(
                search_groups, search_base, target_fpr, thresholds, threshold_provenance
            )
    tokenizer_identity = experiment_specification["tokenizer"]
    records: list[dict[str, Any]] = []
    controlled_threat = {
        "goal": "EVASION",
        "knows_mechanism": True,
        "knows_detector": False,
        "knows_threshold": False,
        "knows_key": False,
        "detector_queries": False,
        "model_logits": False,
        "adaptive": False,
    }
    for task in sorted(selected):
        for member in selected[task]["test"]:
            sample = member.sample
            for runtime in runtimes:
                for length_value in config["lengths"]:
                    length = int(length_value)
                    for label, kind in (
                        ("unwatermarked", "unwatermarked"),
                        ("watermarked", runtime.short_name),
                    ):
                        source = entries[(task, "test", sample.sample_id, kind)]
                        tokens = _tokens(source, length)
                        records.append(
                            _make_record(
                                run_id=run_id,
                                runtime=runtime,
                                tokenizer=tokenizer,
                                tokenizer_identity=tokenizer_identity,
                                thresholds=thresholds,
                                model_revision=model_revision,
                                tokenizer_revision=tokenizer_revision,
                                task=task,
                                sample_id=sample.sample_id,
                                label=label,
                                token_ids=tokens,
                                nominal_generation_length=length,
                                entropy=_entropy(source, length),
                                semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
                                regime="clean",
                                evaluation_kind="clean_detection",
                                transform="clean",
                                transform_strength=0.0,
                                quality={"normalized_token_edit_distance": 0.0},
                                input_sha256=_sha(sample.prompt),
                                marked_fraction=1.0,
                            )
                        )
                    for label, kind in (
                        ("unwatermarked", "unwatermarked"),
                        ("watermarked", runtime.short_name),
                    ):
                        source = entries[(task, "test", sample.sample_id, kind)]
                        original = _tokens(source, length)
                        edit_seed = derive_seed(
                            master_seed,
                            f"pilot/{task}/{runtime.short_name}/{length}/deletion/0.1/test/{sample.sample_id}/{label}",
                        )
                        edited = list(
                            random_deletion(original, deletion_strength, edit_seed).token_ids
                        )
                        original_text = _decode(tokenizer, original)
                        edited_text = _decode(tokenizer, edited)
                        integrity = literal_integrity(original_text, edited_text)
                        quality: dict[str, Any] = {
                            "normalized_token_edit_distance": normalized_token_edit_distance(
                                original, edited
                            ),
                            "literal_recall": integrity.recall,
                        }
                        if task == "gsm8k":
                            quality["final_answer_preserved"] = answer_preserved(
                                original_text, edited_text
                            )
                        for semantics, regime in (
                            (ThresholdSemantics.FIXED_CLEAN_THRESHOLD, "clean"),
                            (
                                ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD,
                                "operation_conditioned",
                            ),
                        ):
                            records.append(
                                _make_record(
                                    run_id=run_id,
                                    runtime=runtime,
                                    tokenizer=tokenizer,
                                    tokenizer_identity=tokenizer_identity,
                                    thresholds=thresholds,
                                    model_revision=model_revision,
                                    tokenizer_revision=tokenizer_revision,
                                    task=task,
                                    sample_id=sample.sample_id,
                                    label=label,
                                    token_ids=edited,
                                    nominal_generation_length=length,
                                    entropy=_entropy(source, length),
                                    semantics=semantics,
                                    regime=regime,
                                    operation=(
                                        "random_deletion"
                                        if semantics
                                        is ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD
                                        else None
                                    ),
                                    operation_strength=(
                                        deletion_strength
                                        if semantics
                                        is ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD
                                        else None
                                    ),
                                    evaluation_kind="operation_held_out",
                                    transform="random_deletion",
                                    transform_strength=deletion_strength,
                                    quality=quality,
                                    input_sha256=canonical_sha256(original),
                                    marked_fraction=1.0,
                                    threat_model=(
                                        controlled_threat if label == "watermarked" else None
                                    ),
                                )
                            )
                marked_entry = entries[(task, "test", sample.sample_id, runtime.short_name)]
                unmarked_entry = entries[(task, "test", sample.sample_id, "unwatermarked")]
                marked = _tokens(marked_entry, document_length)
                unmarked = _tokens(unmarked_entry, document_length)
                for fraction_value in config["mixed_authorship"]["marked_fractions"]:
                    fraction = float(fraction_value)
                    for layout in config["mixed_authorship"]["layouts"]:
                        composed = compose_tokens(
                            marked,
                            unmarked,
                            fraction,
                            separated_segments=layout == "separated",
                            seed=derive_seed(
                                master_seed,
                                f"pilot/{task}/{runtime.short_name}/mix/{fraction}/{layout}/{sample.sample_id}",
                            ),
                            marked_source_id=f"{sample.sample_id}:watermarked",
                            unmarked_source_id=f"{sample.sample_id}:unwatermarked",
                        )
                        truth_spans = tuple(
                            (segment.start, segment.end)
                            for segment in composed.segments
                            if segment.marked
                        )
                        segment_records = [
                            {**asdict(segment), "coordinate_system": CoordinateSystem.TOKEN}
                            for segment in composed.segments
                        ]
                        whole_localization = localization_report(
                            (0, len(composed.token_ids)), truth_spans
                        ).to_dict()
                        for label, document_tokens, localization, segments in (
                            (
                                "unwatermarked",
                                unmarked,
                                None,
                                None,
                            ),
                            (
                                "watermarked",
                                composed.token_ids,
                                whole_localization,
                                segment_records,
                            ),
                        ):
                            records.append(
                                _make_record(
                                    run_id=run_id,
                                    runtime=runtime,
                                    tokenizer=tokenizer,
                                    tokenizer_identity=tokenizer_identity,
                                    thresholds=thresholds,
                                    model_revision=model_revision,
                                    tokenizer_revision=tokenizer_revision,
                                    task=task,
                                    sample_id=sample.sample_id,
                                    label=label,
                                    token_ids=document_tokens,
                                    nominal_generation_length=document_length,
                                    entropy=None,
                                    semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
                                    regime="clean",
                                    evaluation_kind="mixed_document_whole",
                                    transform="mixed_document_whole",
                                    transform_strength=0.0,
                                    quality={},
                                    input_sha256=canonical_sha256([marked, unmarked]),
                                    marked_fraction=composed.realized_marked_fraction,
                                    layout=layout,
                                    localization=localization,
                                    segments=segments,
                                )
                            )
                        positive_score, positive_best = _search_score(
                            runtime, tokenizer, composed.token_ids, search_specification
                        )
                        _ = positive_score
                        window_localization = localization_report(
                            (int(positive_best["start"]), int(positive_best["end"])),
                            truth_spans,
                        ).to_dict()
                        for label, document_tokens, localization, segments in (
                            ("unwatermarked", unmarked, None, None),
                            (
                                "watermarked",
                                composed.token_ids,
                                window_localization,
                                segment_records,
                            ),
                        ):
                            records.append(
                                _make_record(
                                    run_id=run_id,
                                    runtime=runtime,
                                    tokenizer=tokenizer,
                                    tokenizer_identity=tokenizer_identity,
                                    thresholds=thresholds,
                                    model_revision=model_revision,
                                    tokenizer_revision=tokenizer_revision,
                                    task=task,
                                    sample_id=sample.sample_id,
                                    label=label,
                                    token_ids=document_tokens,
                                    nominal_generation_length=document_length,
                                    entropy=None,
                                    semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
                                    regime="window_search",
                                    evaluation_kind="mixed_document_window_search",
                                    transform="mixed_document_window_search",
                                    transform_strength=0.0,
                                    quality={},
                                    input_sha256=canonical_sha256([marked, unmarked]),
                                    marked_fraction=composed.realized_marked_fraction,
                                    layout=layout,
                                    search_specification=search_specification,
                                    localization=localization,
                                    segments=segments,
                                )
                            )
    write_jsonl(run_dir / "examples.jsonl", records)
    aggregates = aggregate_records(records, target_fpr)
    aggregates_json = json.loads(json.dumps(aggregates, default=asdict))
    write_json(run_dir / "aggregate.json", aggregates_json)
    shutil.copyfile(checkpoint_path, run_dir / "generation-checkpoint.json")
    scientific_artifacts = {
        name: _sha((run_dir / name).read_bytes())
        for name in ("generation-checkpoint.json", "examples.jsonl", "aggregate.json")
    }
    threshold_dicts = []
    for threshold in thresholds:
        threshold_dicts.append(
            {**threshold.to_dict(), **threshold_provenance[threshold.threshold_id]}
        )
    manifest: dict[str, Any] = {
        "schema_version": "2.0.0",
        "experiment_id": "waterlarp-v1-cpu-integration-pilot",
        "experiment_spec_id": experiment_spec_id,
        "run_id": run_id,
        "artifact_set_id": artifact_set_id(scientific_artifacts),
        "created_by_tool_version": __version__,
        "git_commit": _git(root, "rev-parse", "HEAD"),
        "git_dirty": bool(_git(root, "status", "--porcelain")),
        "git_diff_sha256": experiment_specification["code_identity"]["git_diff_sha256"],
        "environment": {
            "python_version": sys.version,
            "platform": platform.platform(),
            "cpu": platform.processor(),
            "gpu": [],
            "cuda": None,
            "torch_version": torch.__version__,
            "transformers_version": "5.15.0",
            "datasets_version": "5.0.1",
        },
        "model_repo": model_config["repo"],
        "model_revision": model_revision,
        "model_artifact_hashes": model_artifacts,
        "tokenizer_repo": model_config["repo"],
        "tokenizer_revision": tokenizer_revision,
        "tokenizer_artifact_hashes": tokenizer_artifacts,
        "sample_sets": sample_sets,
        "sample_set_sha256": sample_set_digest(sample_sets),
        "watermark_config": {runtime.name: runtime.adapter.metadata() for runtime in runtimes},
        "authority_record": {runtime.name: runtime.authority.to_dict() for runtime in runtimes},
        "generation_config": generation_config,
        "detector_config": {runtime.name: runtime.adapter.metadata() for runtime in runtimes},
        "calibration_policy": experiment_specification["calibration_policy"],
        "evidence_length_policy": experiment_specification["evidence_length_policy"],
        "threshold_records": threshold_dicts,
        "transform_config": config["transforms"],
        "search_config": asdict(search_specification),
        "rng_seeds": {"master": master_seed},
        "environment_lock_sha256": lock_hash,
        "source_ledger_sha256": source_ledger_hash,
        "checkpoint": {
            "path": "generation-checkpoint.json",
            "payload_sha256": checkpoint["payload_sha256"],
            "file_sha256": scientific_artifacts["generation-checkpoint.json"],
            **checkpoint_identity,
        },
        "scientific_artifact_sha256": scientific_artifacts,
        "experiment_specification": experiment_specification,
        "execution_scope": PILOT_EXECUTION_SCOPE,
        "publication_status": PILOT_PUBLICATION_STATUS,
        "paper_plan_status": PILOT_PAPER_PLAN_STATUS,
        "record_count": len(records),
        "aggregate_count": len(aggregates_json),
        "generation_runtime_seconds_this_invocation": time.perf_counter() - generation_started,
        "fpr_interpretation": (
            "held-out counts are emitted, but tiny N cannot resolve the 1% target"
        ),
    }
    validate_manifest(manifest)
    write_json(run_dir / "manifest.json", manifest)
    checksums = {
        name: _sha((run_dir / name).read_bytes())
        for name in (
            "manifest.json",
            "generation-checkpoint.json",
            "examples.jsonl",
            "aggregate.json",
        )
    }
    write_json(run_dir / "checksums.json", checksums)
    verify_artifact_checksums(run_dir)
    return run_dir
