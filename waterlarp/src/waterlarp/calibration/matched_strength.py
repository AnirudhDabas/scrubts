"""Legitimate one-axis strength matching on a disjoint calibration split."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass


@dataclass(frozen=True)
class StrengthCandidate:
    value: float
    tpr: float
    quality: float


@dataclass(frozen=True)
class MatchedStrengthResult:
    status: str
    selected: StrengthCandidate | None
    candidates: tuple[StrengthCandidate, ...]
    target_tpr: float
    target_fpr: float


def select_weakest_attaining(
    candidates: Iterable[StrengthCandidate], *, target_tpr: float, target_fpr: float
) -> MatchedStrengthResult:
    frozen = tuple(sorted(candidates, key=lambda candidate: candidate.value))
    if not frozen:
        return MatchedStrengthResult("UNSUPPORTED", None, frozen, target_tpr, target_fpr)
    selected = next((candidate for candidate in frozen if candidate.tpr >= target_tpr), None)
    status = "SUPPORTED" if selected else "UNATTAINABLE"
    return MatchedStrengthResult(status, selected, frozen, target_tpr, target_fpr)
