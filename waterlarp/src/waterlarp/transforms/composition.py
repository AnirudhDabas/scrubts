"""Composition transform descriptors."""

from dataclasses import dataclass


@dataclass(frozen=True)
class CompositionTransform:
    marked_fraction: float
    separated_segments: bool
    seed: int

    def validate(self) -> None:
        if not 0 < self.marked_fraction <= 1:
            raise ValueError("marked_fraction must be in (0, 1]")
