import pytest

from waterlarp.quality.general import normalized_token_edit_distance
from waterlarp.quality.gsm8k import answer_preserved
from waterlarp.quality.integrity import literal_integrity
from waterlarp.quality.mbpp import execute_candidate
from waterlarp.transforms.token_edits import random_deletion, random_insertion, random_substitution


def test_token_edits_are_deterministic_and_preserve_input() -> None:
    source = [1, 2, 3, 4, 5, 6]
    original = list(source)
    assert random_deletion(source, 0.5, 7) == random_deletion(source, 0.5, 7)
    assert random_substitution(source, range(20), 0.5, 7) == random_substitution(
        source, range(20), 0.5, 7
    )
    assert random_insertion(source, range(20), 0.5, 7) == random_insertion(
        source, range(20), 0.5, 7
    )
    assert source == original


def test_quality_and_integrity_metrics() -> None:
    assert normalized_token_edit_distance([1, 2, 3], [1, 9, 3]) == 1 / 3
    report = literal_integrity("Use 42 at https://example.test and `x=1`.", "Use 42 and `x=1`.")
    assert report.recall["numbers"] == 1.0
    assert report.recall["urls"] == 0.0
    assert answer_preserved("work #### 1,024", "proof #### 1024") is True


def test_mbpp_execution_is_opt_in_and_reports_assertions() -> None:
    with pytest.raises(PermissionError, match="explicit opt-in"):
        execute_candidate("def add(a, b): return a + b", ("assert add(1, 2) == 3",))
    result = execute_candidate(
        "def add(a, b): return a + b",
        ("assert add(1, 2) == 3", "assert add(-1, 1) == 0"),
        allow_untrusted_execution=True,
    )
    assert result.passed
    assert result.tests_run == 2
    assert "not-os-sandboxed" in result.isolation
