"""Deterministic composition with exact half-open token boundaries."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from waterlarp.rng import python_rng


@dataclass(frozen=True)
class Segment:
    start: int
    end: int
    marked: bool
    source_id: str


@dataclass(frozen=True)
class ComposedDocument:
    token_ids: tuple[int, ...]
    segments: tuple[Segment, ...]
    requested_marked_fraction: float
    realized_marked_fraction: float


def compose_tokens(
    marked: Sequence[int],
    unmarked: Sequence[int],
    fraction: float,
    *,
    separated_segments: bool,
    seed: int,
    marked_source_id: str = "watermarked-generated",
    unmarked_source_id: str = "unwatermarked-generated",
) -> ComposedDocument:
    if not 0 < fraction <= 1:
        raise ValueError("fraction must be in (0, 1]")
    if not marked or not unmarked:
        raise ValueError("both marked and unmarked token sources must be non-empty")
    total = min(len(marked) + len(unmarked), max(len(marked), len(unmarked)))
    marked_count = max(1, min(len(marked), round(total * fraction)))
    unmarked_count = min(len(unmarked), total - marked_count)
    rng = python_rng(seed, "composition")
    if not separated_segments or marked_count < 2:
        before = rng.randrange(unmarked_count + 1)
        pieces = [
            (tuple(unmarked[:before]), False, unmarked_source_id),
            (tuple(marked[:marked_count]), True, marked_source_id),
            (tuple(unmarked[before:unmarked_count]), False, unmarked_source_id),
        ]
    else:
        first = marked_count // 2
        split = max(1, unmarked_count // 2)
        pieces = [
            (tuple(marked[:first]), True, marked_source_id),
            (tuple(unmarked[:split]), False, unmarked_source_id),
            (tuple(marked[first:marked_count]), True, marked_source_id),
            (tuple(unmarked[split:unmarked_count]), False, unmarked_source_id),
        ]
    output: list[int] = []
    segments: list[Segment] = []
    for tokens, is_marked, source in pieces:
        if not tokens:
            continue
        start = len(output)
        output.extend(tokens)
        segments.append(Segment(start, len(output), is_marked, source))
    realized = sum(segment.end - segment.start for segment in segments if segment.marked) / len(
        output
    )
    return ComposedDocument(tuple(output), tuple(segments), fraction, realized)
