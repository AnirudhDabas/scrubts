"""Canonical detector evidence and checkpoint-independent rescoring."""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass
from typing import Any

from waterlarp.adapters.base import DetectionScore
from waterlarp.calibration.thresholds import (
    ThresholdCalibration,
    ThresholdContext,
    ThresholdSelection,
    select_threshold,
    selected_decision,
)
from waterlarp.composition.windows import fixed_window_max
from waterlarp.config import Comparator, ThresholdSemantics
from waterlarp.manifests import canonical_sha256


@dataclass(frozen=True)
class RescoreResult:
    score: float
    p_value: float | None
    scored_unit_count: int
    threshold_id: str | None
    decision: bool | None
    decision_status: str


def detector_config_sha256(metadata: Mapping[str, Any]) -> str:
    return canonical_sha256(dict(metadata))


def _jsonable_detector_value(value: Any) -> Any:
    """Normalize detector-native arrays/tensors without changing their values."""
    if value is None or isinstance(value, (str, bool, int, float)):
        return value
    if isinstance(value, Mapping):
        return {str(key): _jsonable_detector_value(item) for key, item in value.items()}
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        return [_jsonable_detector_value(item) for item in value]
    tolist = getattr(value, "tolist", None)
    if callable(tolist):
        return _jsonable_detector_value(tolist())
    raise TypeError(f"detector evidence contains unsupported {type(value).__name__}")


def _selection_payload(selection: ThresholdSelection) -> dict[str, Any]:
    if selection.threshold is None:
        return {
            "selection_status": selection.status,
            "selection_reason": selection.reason,
            "threshold_id": None,
            "value": None,
            "comparator": None,
        }
    return {
        "selection_status": selection.status,
        "selection_reason": selection.reason,
        "threshold_id": selection.threshold.threshold_id,
        "value": selection.threshold.threshold,
        "comparator": selection.threshold.comparator,
    }


def canonical_detector_evidence(
    *,
    detector_input_token_ids: Sequence[int],
    tokenizer_identity: Mapping[str, Any],
    detector_metadata: Mapping[str, Any],
    key_provenance: str,
    score: DetectionScore,
    threshold_request: ThresholdContext,
    selection: ThresholdSelection,
    decision: bool | None,
    decision_status: str,
    procedure: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "evidence_schema_version": "1.0.0",
        "procedure": "TOKEN_SEQUENCE" if procedure is None else "WINDOW_MAX",
        "detector_input_token_ids": [int(value) for value in detector_input_token_ids],
        "tokenizer_identity": dict(tokenizer_identity),
        "detector_config_identity": detector_config_sha256(detector_metadata),
        "detector_config": dict(detector_metadata),
        "key_provenance": key_provenance,
        "statistic_name": score.statistic_name,
        "raw_score": score.score,
        "p_value": score.p_value,
        "scored_unit_count": score.scored_unit_count,
        "raw_detector_evidence": _jsonable_detector_value(score.raw_evidence),
        "threshold_request_context": asdict(threshold_request),
        "threshold": _selection_payload(selection),
        "decision": decision,
        "decision_status": decision_status,
        "search_procedure": None if procedure is None else dict(procedure),
    }


def context_from_dict(value: Mapping[str, Any]) -> ThresholdContext:
    return ThresholdContext(
        **{
            **dict(value),
            "threshold_semantics": ThresholdSemantics(value["threshold_semantics"]),
            "comparator": Comparator(value["comparator"]),
        }
    )


def _close(left: float | None, right: float | None) -> bool:
    if left is None or right is None:
        return left is right
    return math.isclose(left, right, rel_tol=0.0, abs_tol=1e-12)


def rescore_canonical_record(
    record: Mapping[str, Any],
    *,
    adapters: Mapping[str, Any],
    tokenizer: Any,
    threshold_records: Sequence[Mapping[str, Any]],
) -> RescoreResult:
    evidence = record.get("detector_evidence")
    if not isinstance(evidence, Mapping):
        raise ValueError("record lacks canonical detector evidence")
    scheme = str(record["scheme"])
    try:
        adapter = adapters[scheme]
    except KeyError as exc:
        raise ValueError(f"no adapter supplied for {scheme}") from exc
    token_ids = [int(value) for value in evidence["detector_input_token_ids"]]
    if evidence["procedure"] == "TOKEN_SEQUENCE":
        rescored = adapter.score_token_ids(token_ids, tokenizer)
    elif evidence["procedure"] == "WINDOW_MAX":
        procedure = evidence["search_procedure"]
        if not isinstance(procedure, Mapping):
            raise ValueError("window evidence lacks search procedure")

        def score_window(values: Sequence[int]) -> float:
            return float(adapter.score_token_ids(list(values), tokenizer).score)

        searched = fixed_window_max(
            token_ids,
            score_window,
            int(procedure["window_size"]),
            int(procedure["stride"]),
        )
        expected_best = procedure["best_window"]
        if (
            searched.maximum.start != expected_best["start"]
            or searched.maximum.end != expected_best["end"]
        ):
            raise ValueError("canonical best window is not reproducible")
        rescored = adapter.score_token_ids(
            token_ids[searched.maximum.start : searched.maximum.end], tokenizer
        )
    else:
        raise ValueError("unsupported canonical detector procedure")
    if not _close(rescored.score, float(evidence["raw_score"])):
        raise ValueError("canonical detector raw score mismatch")
    expected_p = evidence["p_value"]
    if not _close(rescored.p_value, None if expected_p is None else float(expected_p)):
        raise ValueError("canonical detector p-value mismatch")
    if rescored.scored_unit_count != int(evidence["scored_unit_count"]):
        raise ValueError("canonical detector scored-unit count mismatch")
    calibrations = tuple(ThresholdCalibration.from_dict(value) for value in threshold_records)
    request = context_from_dict(evidence["threshold_request_context"])
    selection = select_threshold(calibrations, request)
    decision, decision_status = selected_decision(rescored.score, selection)
    expected_threshold_id = evidence["threshold"]["threshold_id"]
    actual_threshold_id = None if selection.threshold is None else selection.threshold.threshold_id
    if actual_threshold_id != expected_threshold_id:
        raise ValueError("canonical threshold lookup mismatch")
    if decision != record["decision"] or decision_status != record["decision_status"]:
        raise ValueError("canonical detector decision mismatch")
    return RescoreResult(
        rescored.score,
        rescored.p_value,
        rescored.scored_unit_count,
        actual_threshold_id,
        decision,
        decision_status,
    )
