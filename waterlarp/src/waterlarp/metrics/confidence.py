"""Binomial confidence reporting with one frozen exact method."""

from __future__ import annotations

from dataclasses import dataclass

from scipy.stats import beta


@dataclass(frozen=True)
class BinomialEstimate:
    successes: int
    trials: int
    point_estimate: float
    confidence_level: float
    interval_method: str
    lower: float
    upper: float


def clopper_pearson(
    successes: int, trials: int, confidence_level: float = 0.95
) -> BinomialEstimate:
    """Exact equal-tailed interval; conservative coverage suits small FPR counts."""
    if trials <= 0 or not 0 <= successes <= trials:
        raise ValueError("require 0 <= successes <= trials and trials > 0")
    if not 0 < confidence_level < 1:
        raise ValueError("confidence_level must be between zero and one")
    alpha = 1 - confidence_level
    lower = 0.0 if successes == 0 else float(beta.ppf(alpha / 2, successes, trials - successes + 1))
    upper = (
        1.0
        if successes == trials
        else float(beta.ppf(1 - alpha / 2, successes + 1, trials - successes))
    )
    return BinomialEstimate(
        successes, trials, successes / trials, confidence_level, "Clopper-Pearson", lower, upper
    )
