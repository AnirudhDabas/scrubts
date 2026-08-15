"""Conditioned, comparator-explicit threshold calibration and lookup."""

from __future__ import annotations

import math
from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass, replace
from typing import Any

from waterlarp.config import Comparator, DecisionStatus, ThresholdSemantics
from waterlarp.manifests import canonical_json_bytes, sha256_hex


@dataclass(frozen=True)
class ThresholdContext:
    scheme: str
    mechanism_version: str
    detector_config_sha256: str
    model_revision: str
    tokenizer_revision: str
    task: str
    key_policy: str
    threshold_semantics: ThresholdSemantics
    evidence_length_policy: str
    scored_unit_count: int
    regime: str
    operation: str | None
    operation_strength: float | None
    search_spec_sha256: str | None
    comparator: Comparator
    comparator_authority: str

    def validate(self) -> None:
        strings = (
            self.scheme,
            self.mechanism_version,
            self.detector_config_sha256,
            self.model_revision,
            self.tokenizer_revision,
            self.task,
            self.key_policy,
            self.evidence_length_policy,
            self.regime,
            self.comparator_authority,
        )
        if any(not value for value in strings):
            raise ValueError("threshold conditioning dimensions must not be empty")
        if self.scored_unit_count < 0:
            raise ValueError("scored_unit_count must be non-negative")
        conditioned = self.threshold_semantics is ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD
        if conditioned != (self.operation is not None and self.operation_strength is not None):
            raise ValueError("operation-conditioned contexts require operation and strength")
        if self.regime not in {"clean", "operation_conditioned", "window_search"}:
            raise ValueError("unsupported threshold regime")

    @classmethod
    def fixture(cls, comparator: Comparator) -> ThresholdContext:
        return cls(
            scheme="fixture",
            mechanism_version="fixture-1",
            detector_config_sha256="0" * 64,
            model_revision="fixture-model",
            tokenizer_revision="fixture-tokenizer",
            task="fixture-task",
            key_policy="fixture-key",
            threshold_semantics=ThresholdSemantics.FIXED_CLEAN_THRESHOLD,
            evidence_length_policy="exact_scored_units_v1",
            scored_unit_count=1,
            regime="clean",
            operation=None,
            operation_strength=None,
            search_spec_sha256=None,
            comparator=comparator,
            comparator_authority="test fixture",
        )

    def identity_object(self) -> dict[str, Any]:
        self.validate()
        return asdict(self)


@dataclass(frozen=True)
class ThresholdCalibration:
    threshold_id: str
    context: ThresholdContext
    threshold: float
    target_fpr: float
    negative_count: int
    maximum_allowed_false_positives: int
    false_positive_count: int
    empirical_calibration_exceedance: float
    resolution_status: str

    @property
    def comparator(self) -> Comparator:
        return self.context.comparator

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, value: Mapping[str, Any]) -> ThresholdCalibration:
        context_value = value["context"]
        if not isinstance(context_value, Mapping):
            raise TypeError("threshold context must be an object")
        context = ThresholdContext(
            **{
                **dict(context_value),
                "threshold_semantics": ThresholdSemantics(context_value["threshold_semantics"]),
                "comparator": Comparator(context_value["comparator"]),
            }
        )
        return cls(
            threshold_id=str(value["threshold_id"]),
            context=context,
            threshold=float(value["threshold"]),
            target_fpr=float(value["target_fpr"]),
            negative_count=int(value["negative_count"]),
            maximum_allowed_false_positives=int(value["maximum_allowed_false_positives"]),
            false_positive_count=int(value["false_positive_count"]),
            empirical_calibration_exceedance=float(value["empirical_calibration_exceedance"]),
            resolution_status=str(value["resolution_status"]),
        )


@dataclass(frozen=True)
class ThresholdSelection:
    status: str
    threshold: ThresholdCalibration | None
    observed_scored_unit_count: int
    reason: str


def compare(score: float, threshold: float, comparator: Comparator) -> bool:
    if comparator is Comparator.STRICT_GREATER:
        return score > threshold
    if comparator is Comparator.GREATER_OR_EQUAL:
        return score >= threshold
    raise ValueError(f"unsupported comparator: {comparator}")


def _conservative_threshold(
    ordered: tuple[float, ...], allowed: int, comparator: Comparator
) -> float:
    if comparator is Comparator.STRICT_GREATER:
        # With strict comparison, the value at index ``allowed`` excludes itself
        # and every tie at that value. At the end, the minimum permits all N.
        return ordered[allowed] if allowed < len(ordered) else ordered[-1]
    if allowed == 0:
        return math.nextafter(ordered[0], math.inf)
    candidate = ordered[allowed - 1]
    inclusive_count = sum(value >= candidate for value in ordered)
    return candidate if inclusive_count <= allowed else math.nextafter(candidate, math.inf)


def calibrate_threshold(
    negative_scores: Iterable[float],
    target_fpr: float,
    *,
    context: ThresholdContext | None = None,
    comparator: Comparator = Comparator.GREATER_OR_EQUAL,
) -> ThresholdCalibration:
    if not 0 < target_fpr < 1:
        raise ValueError("target_fpr must be between zero and one")
    values = tuple(float(score) for score in negative_scores)
    if not values or any(not math.isfinite(score) for score in values):
        raise ValueError("negative_scores must contain finite values")
    resolved_context = context or ThresholdContext.fixture(comparator)
    resolved_context.validate()
    if resolved_context.scored_unit_count == 0:
        raise ValueError("a threshold cannot be calibrated without scored detector units")
    if resolved_context.comparator is not comparator:
        raise ValueError("context and calibration comparator must agree")
    allowed = math.floor(target_fpr * len(values))
    ordered = tuple(sorted(values, reverse=True))
    threshold = _conservative_threshold(ordered, allowed, comparator)
    false_positive_count = sum(compare(score, threshold, comparator) for score in values)
    if false_positive_count > allowed:
        raise AssertionError("conservative calibration exceeded its false-positive allowance")
    identity = {
        "context": resolved_context.identity_object(),
        "target_fpr": target_fpr,
        "threshold": threshold,
        "negative_count": len(values),
        "false_positive_count": false_positive_count,
    }
    return ThresholdCalibration(
        threshold_id=f"wlrt1-{sha256_hex(canonical_json_bytes(identity))[:24]}",
        context=resolved_context,
        threshold=threshold,
        target_fpr=target_fpr,
        negative_count=len(values),
        maximum_allowed_false_positives=allowed,
        false_positive_count=false_positive_count,
        empirical_calibration_exceedance=false_positive_count / len(values),
        resolution_status="RESOLVED" if len(values) * target_fpr >= 1 else "UNRESOLVED",
    )


def decisions(scores: Iterable[float], calibration: ThresholdCalibration) -> tuple[bool, ...]:
    return tuple(
        compare(float(score), calibration.threshold, calibration.comparator) for score in scores
    )


def _without_length(context: ThresholdContext) -> dict[str, Any]:
    value = context.identity_object()
    del value["scored_unit_count"]
    return value


def select_threshold(
    calibrations: Iterable[ThresholdCalibration], request: ThresholdContext
) -> ThresholdSelection:
    """Select by observable scored units; no original/nominal length is accepted."""

    request.validate()
    frozen = tuple(calibrations)
    exact = [calibration for calibration in frozen if calibration.context == request]
    if len(exact) == 1:
        return ThresholdSelection("SELECTED", exact[0], request.scored_unit_count, "exact match")
    if len(exact) > 1:
        raise ValueError("duplicate threshold identities for one conditioning context")
    comparable = [
        calibration
        for calibration in frozen
        if _without_length(calibration.context) == _without_length(request)
    ]
    if not comparable:
        return ThresholdSelection(
            "UNRESOLVED", None, request.scored_unit_count, "no threshold for exact conditioning"
        )
    supported = sorted({item.context.scored_unit_count for item in comparable})
    if request.scored_unit_count < supported[0]:
        return ThresholdSelection(
            "UNSUPPORTED",
            None,
            request.scored_unit_count,
            "observed evidence is below minimum calibrated support",
        )
    if request.scored_unit_count > supported[-1]:
        return ThresholdSelection(
            "UNRESOLVED",
            None,
            request.scored_unit_count,
            "observed evidence exceeds calibrated support",
        )
    return ThresholdSelection(
        "UNRESOLVED",
        None,
        request.scored_unit_count,
        "observed evidence length lies between exact calibrated lengths",
    )


def selected_decision(score: float, selection: ThresholdSelection) -> tuple[bool | None, str]:
    if selection.threshold is None:
        status = (
            DecisionStatus.UNSUPPORTED
            if selection.status == "UNSUPPORTED"
            else DecisionStatus.UNRESOLVED
        )
        return None, status.value
    decision = compare(score, selection.threshold.threshold, selection.threshold.comparator)
    return decision, (
        DecisionStatus.DETECTED.value if decision else DecisionStatus.NOT_DETECTED.value
    )


def length_specific_calibration(
    negatives_by_length: Mapping[int, list[float]],
    target_fpr: float,
    *,
    context: ThresholdContext,
) -> dict[int, ThresholdCalibration]:
    if any(length <= 0 for length in negatives_by_length):
        raise ValueError("evidence lengths must be positive")
    return {
        length: calibrate_threshold(
            scores,
            target_fpr,
            context=replace(context, scored_unit_count=length),
            comparator=context.comparator,
        )
        for length, scores in sorted(negatives_by_length.items())
    }
