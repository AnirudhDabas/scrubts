"""Shannon and EntroBench-compatible spike entropy over base-model logits."""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import cast

import numpy as np
from numpy.typing import NDArray

ENTROBENCH_Z = math.tanh(1.0)  # 0.7615941589914151 in the pinned source.


@dataclass(frozen=True)
class EntropySummary:
    mean: float
    median: float
    quantiles: dict[str, float]


def probabilities_from_logits(logits: NDArray[np.floating]) -> NDArray[np.float64]:
    values = np.asarray(logits, dtype=np.float64)
    shifted = values - np.max(values, axis=-1, keepdims=True)
    exp = np.exp(shifted)
    return cast(NDArray[np.float64], exp / exp.sum(axis=-1, keepdims=True))


def shannon_entropy_from_logits(logits: NDArray[np.floating]) -> NDArray[np.float64]:
    probabilities = probabilities_from_logits(logits)
    terms = np.where(probabilities > 0, probabilities * np.log(probabilities), 0.0)
    return cast(NDArray[np.float64], -terms.sum(axis=-1))


def spike_entropy_from_logits(
    logits: NDArray[np.floating], z: float = ENTROBENCH_Z
) -> NDArray[np.float64]:
    if z < 0:
        raise ValueError("z must be non-negative")
    probabilities = probabilities_from_logits(logits)
    return cast(NDArray[np.float64], (probabilities / (1 + z * probabilities)).sum(axis=-1))


def summarize(values: NDArray[np.floating]) -> EntropySummary:
    array = np.asarray(values, dtype=np.float64)
    if array.size == 0 or not np.all(np.isfinite(array)):
        raise ValueError("entropy values must be finite and non-empty")
    return EntropySummary(
        float(np.mean(array)),
        float(np.median(array)),
        {str(q): float(np.quantile(array, q)) for q in (0.05, 0.25, 0.75, 0.95)},
    )
