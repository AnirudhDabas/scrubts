"""Tokenizer-level edit distance independent of an embedding model."""

from __future__ import annotations

from collections.abc import Sequence


def token_edit_distance(left: Sequence[int], right: Sequence[int]) -> int:
    previous = list(range(len(right) + 1))
    for i, left_token in enumerate(left, start=1):
        current = [i]
        for j, right_token in enumerate(right, start=1):
            current.append(
                min(current[-1] + 1, previous[j] + 1, previous[j - 1] + (left_token != right_token))
            )
        previous = current
    return previous[-1]


def normalized_token_edit_distance(left: Sequence[int], right: Sequence[int]) -> float:
    denominator = max(len(left), len(right))
    return 0.0 if denominator == 0 else token_edit_distance(left, right) / denominator
