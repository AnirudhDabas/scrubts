"""Typed half-open token-coordinate localization metrics."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import asdict, dataclass

from waterlarp.config import CoordinateSystem


def _validate_span(span: tuple[int, int]) -> None:
    start, end = span
    if not 0 <= start <= end:
        raise ValueError("spans must be ordered, non-negative, and half-open")


def span_iou(predicted: tuple[int, int], truth: tuple[int, int]) -> float:
    _validate_span(predicted)
    _validate_span(truth)
    ps, pe = predicted
    ts, te = truth
    intersection = max(0, min(pe, te) - max(ps, ts))
    union = max(pe, te) - min(ps, ts)
    return 1.0 if union == 0 else intersection / union


def best_iou(predicted: tuple[int, int], truths: Iterable[tuple[int, int]]) -> float:
    values = tuple(span_iou(predicted, truth) for truth in truths)
    return max(values, default=0.0)


@dataclass(frozen=True)
class LocalizationReport:
    coordinate_system: CoordinateSystem
    marked_span_definition: str
    marked_spans: tuple[dict[str, int], ...]
    predicted_span: dict[str, int]
    overlap_token_count: int
    union_token_count: int
    iou: float
    start_offset_error: int
    end_offset_error: int

    def to_dict(self) -> dict[str, object]:
        return asdict(self)


def localization_report(
    predicted: tuple[int, int],
    truths: Iterable[tuple[int, int]],
    *,
    coordinate_system: CoordinateSystem = CoordinateSystem.TOKEN,
) -> LocalizationReport:
    if coordinate_system is not CoordinateSystem.TOKEN:
        raise ValueError("WaterLARP v1 localization supports TOKEN coordinates only")
    _validate_span(predicted)
    spans = tuple(sorted(truths))
    if not spans:
        raise ValueError("at least one marked truth span is required")
    for span in spans:
        _validate_span(span)
    if any(left[1] > right[0] for left, right in zip(spans, spans[1:], strict=False)):
        raise ValueError("marked truth spans must not overlap")
    ps, pe = predicted
    overlaps = tuple(max(0, min(pe, end) - max(ps, start)) for start, end in spans)
    overlap = sum(overlaps)
    truth_size = sum(end - start for start, end in spans)
    predicted_size = pe - ps
    union = truth_size + predicted_size - overlap
    matched_index = max(
        range(len(spans)),
        key=lambda index: (overlaps[index], -abs(ps - spans[index][0]), -index),
    )
    matched = spans[matched_index]
    return LocalizationReport(
        coordinate_system,
        "union of exact half-open marked source segments",
        tuple({"start": start, "end": end} for start, end in spans),
        {"start": ps, "end": pe},
        overlap,
        union,
        1.0 if union == 0 else overlap / union,
        ps - matched[0],
        pe - matched[1],
    )
