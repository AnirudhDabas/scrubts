"""Survivability-damage representations without an ambiguous attack score."""

from collections.abc import Iterable
from dataclasses import dataclass


@dataclass(frozen=True)
class FrontierPoint:
    transform_strength: float
    damage: float
    detection_survival: float
    sample_count: int


def pareto_frontier(points: Iterable[FrontierPoint]) -> tuple[FrontierPoint, ...]:
    candidates = sorted(points, key=lambda point: (point.damage, -point.detection_survival))
    frontier: list[FrontierPoint] = []
    best_survival = float("-inf")
    for point in candidates:
        if not 0 <= point.damage <= 1 or not 0 <= point.detection_survival <= 1:
            raise ValueError("damage and detection_survival must be in [0, 1]")
        if point.detection_survival > best_survival:
            frontier.append(point)
            best_survival = point.detection_survival
    return tuple(frontier)
