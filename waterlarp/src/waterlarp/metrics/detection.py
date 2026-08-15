"""Detection metrics with explicit empirical target-FPR resolution."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass

from waterlarp.metrics.confidence import BinomialEstimate, clopper_pearson


@dataclass(frozen=True)
class RateReport:
    status: str
    false_positive_count: int
    negative_count: int
    fpr: BinomialEstimate
    true_positive_count: int
    positive_count: int
    tpr: BinomialEstimate
    target_fpr: float
    estimate_role: str = "HELD_OUT_TEST"


def detection_rates(
    positive_decisions: Iterable[bool], negative_decisions: Iterable[bool], target_fpr: float
) -> RateReport:
    positives = tuple(positive_decisions)
    negatives = tuple(negative_decisions)
    if not positives or not negatives:
        raise ValueError("positive and negative decisions must be non-empty")
    tp = sum(positives)
    fp = sum(negatives)
    # With N negatives, the empirical grid is multiples of 1/N. Even zero
    # observed false positives cannot resolve a target below that grid.
    status = "RESOLVED" if len(negatives) * target_fpr >= 1 else "UNRESOLVED"
    return RateReport(
        status,
        fp,
        len(negatives),
        clopper_pearson(fp, len(negatives)),
        tp,
        len(positives),
        clopper_pearson(tp, len(positives)),
        target_fpr,
    )


def length_at_target(tpr_by_fixed_length: dict[int, float], target_tpr: float) -> int | None:
    return next(
        (length for length, tpr in sorted(tpr_by_fixed_length.items()) if tpr >= target_tpr),
        None,
    )
