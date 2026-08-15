"""Fixed window maximum with procedure-level negative calibration."""

from __future__ import annotations

from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass

from waterlarp.calibration.thresholds import (
    ThresholdCalibration,
    ThresholdContext,
    calibrate_threshold,
)
from waterlarp.config import Comparator
from waterlarp.manifests import canonical_sha256


@dataclass(frozen=True)
class WindowSearchSpecification:
    coordinate_system: str
    document_token_count: int
    window_size: int
    stride: int
    valid_window_policy: str
    detector_evidence_policy: str

    def validate(self) -> None:
        if self.coordinate_system != "TOKEN":
            raise ValueError("window search supports TOKEN coordinates only")
        if self.window_size <= 0 or self.stride <= 0:
            raise ValueError("window size and stride must be positive")
        if self.document_token_count < self.window_size:
            raise ValueError("document must contain at least one complete window")

    @property
    def search_spec_id(self) -> str:
        self.validate()
        return f"wlrx1-{canonical_sha256(self.__dict__)[:24]}"

    @property
    def sha256(self) -> str:
        self.validate()
        return canonical_sha256(self.__dict__)


@dataclass(frozen=True)
class WindowScore:
    start: int
    end: int
    score: float


@dataclass(frozen=True)
class WindowSearchResult:
    maximum: WindowScore
    window_size: int
    stride: int
    windows_searched: int


def fixed_window_max(
    token_ids: Sequence[int], score: Callable[[Sequence[int]], float], window_size: int, stride: int
) -> WindowSearchResult:
    if window_size <= 0 or stride <= 0 or len(token_ids) < window_size:
        raise ValueError("require positive window/stride and a document at least one window long")
    windows = tuple(
        WindowScore(
            start, start + window_size, float(score(token_ids[start : start + window_size]))
        )
        for start in range(0, len(token_ids) - window_size + 1, stride)
    )
    maximum = max(windows, key=lambda window: (window.score, -window.start))
    return WindowSearchResult(maximum, window_size, stride, len(windows))


def calibrate_window_search(
    negative_documents: Iterable[Sequence[int]],
    score: Callable[[Sequence[int]], float],
    *,
    window_size: int,
    stride: int,
    target_document_fpr: float,
    context: ThresholdContext | None = None,
    comparator: Comparator = Comparator.GREATER_OR_EQUAL,
) -> ThresholdCalibration:
    maxima = [
        fixed_window_max(document, score, window_size, stride).maximum.score
        for document in negative_documents
    ]
    return calibrate_threshold(maxima, target_document_fpr, context=context, comparator=comparator)
