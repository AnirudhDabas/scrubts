"""Small deterministic dataset boundary."""

from __future__ import annotations

import hashlib
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Any, Protocol

from waterlarp.manifests import canonical_json_bytes


@dataclass(frozen=True)
class DatasetSample:
    sample_id: str
    prompt: str
    reference: str | None
    metadata: Mapping[str, Any]


@dataclass(frozen=True)
class DatasetIdentity:
    repo: str
    revision: str
    split: str
    license: str
    task: str
    prompt_template: str
    max_generation_tokens: int

    @property
    def prompt_template_sha256(self) -> str:
        return hashlib.sha256(self.prompt_template.encode()).hexdigest()


class DatasetAdapter(Protocol):
    identity: DatasetIdentity

    def samples(self, raw: Iterable[Mapping[str, Any]]) -> tuple[DatasetSample, ...]: ...


def sample_ids_sha256(samples: Iterable[DatasetSample]) -> str:
    return hashlib.sha256(
        canonical_json_bytes([sample.sample_id for sample in samples])
    ).hexdigest()


def assign_splits(sample_ids: Iterable[str], seed: int) -> dict[str, tuple[str, ...]]:
    buckets: dict[str, list[str]] = {"generation": [], "calibration": [], "test": []}
    names = tuple(buckets)
    for sample_id in sorted(sample_ids):
        digest = hashlib.sha256(f"waterlarp-split-v1\0{seed}\0{sample_id}".encode()).digest()
        buckets[names[int.from_bytes(digest[:8], "big") % 3]].append(sample_id)
    if any(not values for values in buckets.values()):
        raise ValueError("sample population is too small for three non-empty deterministic splits")
    return {name: tuple(values) for name, values in buckets.items()}


def fixed_count_splits(
    sample_ids: Iterable[str], *, per_split: int, seed: int
) -> dict[str, tuple[str, ...]]:
    """Choose equal deterministic split sizes and leave excess IDs unused."""
    if per_split <= 0:
        raise ValueError("per_split must be positive")
    supplied = tuple(sample_ids)
    if len(supplied) != len(set(supplied)):
        raise ValueError("duplicate source sample IDs are forbidden")
    ranked = sorted(
        supplied,
        key=lambda sample_id: hashlib.sha256(
            f"waterlarp-fixed-split-v1\0{seed}\0{sample_id}".encode()
        ).digest(),
    )
    required = 3 * per_split
    if len(ranked) < required:
        raise ValueError(f"need at least {required} unique sample IDs")
    return {
        "generation": tuple(ranked[:per_split]),
        "calibration": tuple(ranked[per_split : 2 * per_split]),
        "test": tuple(ranked[2 * per_split : required]),
    }
