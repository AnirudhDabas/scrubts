"""Seeded nonadaptive token edits with an explicit replacement distribution."""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass
from enum import StrEnum

from waterlarp.rng import python_rng


class TokenEditKind(StrEnum):
    DELETION = "random_deletion"
    SUBSTITUTION = "random_substitution"
    INSERTION = "random_insertion"
    SPAN_DELETION = "span_deletion"
    SPAN_SUBSTITUTION = "span_substitution"


@dataclass(frozen=True)
class TokenEditResult:
    token_ids: tuple[int, ...]
    requested_rate: float
    operation_count: int
    source_token_count: int
    kind: TokenEditKind
    seed: int
    replacement_distribution: str

    @property
    def realized_operation_rate(self) -> float:
        return self.operation_count / self.source_token_count if self.source_token_count else 0.0


def _operation_count(length: int, rate: float) -> int:
    if not 0 <= rate <= 1:
        raise ValueError("rate must be in [0, 1]")
    return min(length, math.floor(length * rate + 0.5))


def _replacements(vocabulary: Sequence[int], source: set[int]) -> tuple[int, ...]:
    values = tuple(dict.fromkeys(int(token) for token in vocabulary if int(token) not in source))
    if not values:
        raise ValueError("replacement vocabulary must contain tokens outside the edited source set")
    return values


def random_deletion(token_ids: Sequence[int], rate: float, seed: int) -> TokenEditResult:
    rng = python_rng(seed, TokenEditKind.DELETION)
    count = _operation_count(len(token_ids), rate)
    deleted = set(rng.sample(range(len(token_ids)), count))
    output = tuple(token for index, token in enumerate(token_ids) if index not in deleted)
    return TokenEditResult(
        output, rate, count, len(token_ids), TokenEditKind.DELETION, seed, "not-applicable"
    )


def random_substitution(
    token_ids: Sequence[int], vocabulary: Sequence[int], rate: float, seed: int
) -> TokenEditResult:
    rng = python_rng(seed, TokenEditKind.SUBSTITUTION)
    count = _operation_count(len(token_ids), rate)
    positions = rng.sample(range(len(token_ids)), count)
    output = list(token_ids)
    for position in positions:
        candidates = _replacements(vocabulary, {output[position]})
        output[position] = rng.choice(candidates)
    return TokenEditResult(
        tuple(output),
        rate,
        count,
        len(token_ids),
        TokenEditKind.SUBSTITUTION,
        seed,
        "uniform over supplied vocabulary excluding the source token",
    )


def random_insertion(
    token_ids: Sequence[int], vocabulary: Sequence[int], rate: float, seed: int
) -> TokenEditResult:
    rng = python_rng(seed, TokenEditKind.INSERTION)
    count = _operation_count(len(token_ids), rate)
    if not vocabulary:
        raise ValueError("vocabulary must not be empty")
    output = list(token_ids)
    for position in sorted((rng.randrange(len(token_ids) + 1) for _ in range(count)), reverse=True):
        output.insert(position, rng.choice(tuple(vocabulary)))
    return TokenEditResult(
        tuple(output),
        rate,
        count,
        len(token_ids),
        TokenEditKind.INSERTION,
        seed,
        "uniform over supplied vocabulary",
    )


def contiguous_span_edit(
    token_ids: Sequence[int], vocabulary: Sequence[int], rate: float, seed: int, *, substitute: bool
) -> TokenEditResult:
    count = _operation_count(len(token_ids), rate)
    kind = TokenEditKind.SPAN_SUBSTITUTION if substitute else TokenEditKind.SPAN_DELETION
    if count == 0:
        return TokenEditResult(tuple(token_ids), rate, 0, len(token_ids), kind, seed, "not-applied")
    rng = python_rng(seed, kind)
    start = rng.randrange(len(token_ids) - count + 1)
    output = list(token_ids)
    if substitute:
        for position in range(start, start + count):
            output[position] = rng.choice(_replacements(vocabulary, {output[position]}))
        distribution = "uniform over supplied vocabulary excluding each source token"
    else:
        del output[start : start + count]
        distribution = "not-applicable"
    return TokenEditResult(tuple(output), rate, count, len(token_ids), kind, seed, distribution)
