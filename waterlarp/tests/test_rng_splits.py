import pytest

from waterlarp.config import SplitIds
from waterlarp.datasets.base import assign_splits, fixed_count_splits
from waterlarp.rng import benchmark_key, derive_seed


def test_seed_derivation_is_namespaced_and_stable() -> None:
    assert derive_seed(7, "a") == derive_seed(7, "a")
    assert derive_seed(7, "a") != derive_seed(7, "b")
    assert benchmark_key(7, "kgw") != 15485863


def test_splits_are_deterministic_and_disjoint() -> None:
    ids = [f"sample-{index}" for index in range(30)]
    split = assign_splits(ids, 42)
    assert split == assign_splits(reversed(ids), 42)
    typed = SplitIds(split["generation"], split["calibration"], split["test"])
    typed.validate()


def test_split_overlap_is_rejected() -> None:
    with pytest.raises(ValueError, match="disjoint"):
        SplitIds(("a",), ("a",), ("b",)).validate()


def test_duplicate_ids_within_split_and_source_population_are_rejected() -> None:
    with pytest.raises(ValueError, match="duplicate"):
        SplitIds(("a", "a"), ("b",), ("c",)).validate()
    with pytest.raises(ValueError, match="duplicate"):
        fixed_count_splits(("a", "a", "b", "c"), per_split=1, seed=7)


def test_fixed_count_splits_have_requested_sizes() -> None:
    split = fixed_count_splits((f"id-{index}" for index in range(12)), per_split=2, seed=7)
    assert {name: len(ids) for name, ids in split.items()} == {
        "generation": 2,
        "calibration": 2,
        "test": 2,
    }
