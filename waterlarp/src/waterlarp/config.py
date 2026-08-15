"""Typed experiment semantics that must never be inferred from result values."""

from dataclasses import dataclass
from enum import StrEnum

PILOT_EXECUTION_SCOPE = "INTEGRATION_PILOT"
PILOT_PUBLICATION_STATUS = "PILOT_NOT_BENCHMARK_EVIDENCE"
PILOT_PAPER_PLAN_STATUS = "NOT_EXECUTED"


class ThresholdSemantics(StrEnum):
    FIXED_CLEAN_THRESHOLD = "fixed_clean_threshold"
    OPERATION_CONDITIONED_THRESHOLD = "operation_conditioned_threshold"


class Comparator(StrEnum):
    STRICT_GREATER = ">"
    GREATER_OR_EQUAL = ">="


class DecisionStatus(StrEnum):
    DETECTED = "DETECTED"
    NOT_DETECTED = "NOT_DETECTED"
    UNRESOLVED = "UNRESOLVED"
    UNSUPPORTED = "UNSUPPORTED"


class CoordinateSystem(StrEnum):
    TOKEN = "TOKEN"


class ExperimentMode(StrEnum):
    AUTHORITATIVE_DEFAULT = "authoritative_default"
    MATCHED_STRENGTH = "matched_strength"


class ComparisonStatus(StrEnum):
    SUPPORTED = "SUPPORTED"
    UNSUPPORTED = "UNSUPPORTED"
    UNATTAINABLE = "UNATTAINABLE"


class ThreatGoal(StrEnum):
    EVASION = "EVASION"
    SPOOFING = "SPOOFING"


@dataclass(frozen=True)
class ThreatModel:
    goal: ThreatGoal
    knows_mechanism: bool
    knows_detector: bool
    knows_threshold: bool
    knows_key: bool
    detector_queries: bool
    model_logits: bool
    adaptive: bool

    @classmethod
    def controlled_token_edit(cls) -> "ThreatModel":
        return cls(ThreatGoal.EVASION, True, False, False, False, False, False, False)


@dataclass(frozen=True)
class SplitIds:
    generation: tuple[str, ...]
    calibration: tuple[str, ...]
    test: tuple[str, ...]

    def validate(self) -> None:
        values = [self.generation, self.calibration, self.test]
        if any(len(group) != len(set(group)) for group in values):
            raise ValueError("duplicate sample IDs within a split are forbidden")
        groups = [set(group) for group in values]
        if any(len(group) == 0 for group in groups):
            raise ValueError("all splits must be non-empty")
        if groups[0] & groups[1] or groups[0] & groups[2] or groups[1] & groups[2]:
            raise ValueError("generation, calibration, and test splits must be disjoint")
