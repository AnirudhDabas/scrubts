from dataclasses import replace

import pytest

from waterlarp.calibration.matched_strength import StrengthCandidate, select_weakest_attaining
from waterlarp.calibration.thresholds import (
    ThresholdContext,
    calibrate_threshold,
    decisions,
    select_threshold,
)
from waterlarp.config import Comparator, ThresholdSemantics
from waterlarp.metrics.confidence import clopper_pearson
from waterlarp.metrics.detection import detection_rates, length_at_target


def context(comparator: Comparator = Comparator.STRICT_GREATER) -> ThresholdContext:
    return ThresholdContext.fixture(comparator)


@pytest.mark.parametrize(
    ("comparator", "expected_threshold"),
    ((Comparator.STRICT_GREATER, 98.0), (Comparator.GREATER_OR_EQUAL, 99.0)),
)
def test_distinct_scores_at_one_percent_record_one_real_false_positive(
    comparator: Comparator, expected_threshold: float
) -> None:
    calibration = calibrate_threshold(
        range(100), 0.01, context=context(comparator), comparator=comparator
    )
    assert calibration.threshold == expected_threshold
    assert calibration.false_positive_count == 1
    assert calibration.empirical_calibration_exceedance == 0.01
    assert sum(decisions(range(100), calibration)) == 1


@pytest.mark.parametrize("comparator", tuple(Comparator))
def test_ties_at_cutoff_and_all_equal_are_conservative(comparator: Comparator) -> None:
    tied = [9.0, 9.0, *([0.0] * 98)]
    calibration = calibrate_threshold(
        tied, 0.01, context=context(comparator), comparator=comparator
    )
    assert calibration.false_positive_count == 0
    assert sum(decisions(tied, calibration)) == 0
    equal = calibrate_threshold(
        [4.0] * 100, 0.01, context=context(comparator), comparator=comparator
    )
    assert equal.false_positive_count == 0


def test_tiny_n_and_sub_resolution_target_remain_unresolved() -> None:
    calibration = calibrate_threshold(
        [1.0, 2.0],
        0.01,
        context=context(Comparator.STRICT_GREATER),
        comparator=Comparator.STRICT_GREATER,
    )
    assert calibration.resolution_status == "UNRESOLVED"
    assert calibration.false_positive_count == 0
    assert detection_rates([True], [False] * 99, 0.01).status == "UNRESOLVED"
    assert detection_rates([True], [False] * 100, 0.01).status == "RESOLVED"


def test_task_config_key_evidence_and_operation_conditioning_are_exact() -> None:
    c4 = context(Comparator.STRICT_GREATER)
    c4_calibration = calibrate_threshold(range(100), 0.01, context=c4, comparator=c4.comparator)
    for mismatch in (
        replace(c4, task="mbpp"),
        replace(c4, key_policy="different-key"),
        replace(c4, detector_config_sha256="f" * 64),
        replace(c4, scored_unit_count=2),
    ):
        assert select_threshold([c4_calibration], mismatch).threshold is None
    conditioned = replace(
        c4,
        threshold_semantics=ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD,
        regime="operation_conditioned",
        operation="random_deletion",
        operation_strength=0.1,
    )
    conditioned_calibration = calibrate_threshold(
        range(100), 0.01, context=conditioned, comparator=conditioned.comparator
    )
    assert select_threshold([conditioned_calibration], conditioned).threshold is not None
    assert (
        select_threshold(
            [conditioned_calibration], replace(conditioned, operation_strength=0.2)
        ).threshold
        is None
    )
    assert (
        select_threshold(
            [conditioned_calibration], replace(conditioned, operation="random_insertion")
        ).threshold
        is None
    )


def test_shifted_task_distributions_cannot_be_silently_pooled() -> None:
    c4 = context(Comparator.STRICT_GREATER)
    mbpp = replace(c4, task="mbpp")
    c4_calibration = calibrate_threshold(range(100), 0.01, context=c4, comparator=c4.comparator)
    mbpp_calibration = calibrate_threshold(
        range(100, 200), 0.01, context=mbpp, comparator=mbpp.comparator
    )
    assert c4_calibration.threshold != mbpp_calibration.threshold
    assert select_threshold([c4_calibration], mbpp).threshold is None
    assert select_threshold([mbpp_calibration], c4).threshold is None


def test_zero_false_positives_has_nonzero_exact_upper_bound() -> None:
    estimate = clopper_pearson(0, 100)
    assert estimate.point_estimate == 0
    assert 0.03 < estimate.upper < 0.04
    assert estimate.interval_method == "Clopper-Pearson"


def test_matched_strength_statuses_and_fixed_bucket_length() -> None:
    result = select_weakest_attaining(
        [StrengthCandidate(1.0, 0.9, 0.99), StrengthCandidate(2.0, 0.99, 0.95)],
        target_tpr=0.98,
        target_fpr=0.01,
    )
    assert result.selected and result.selected.value == 2.0
    assert select_weakest_attaining([], target_tpr=0.98, target_fpr=0.01).status == "UNSUPPORTED"
    assert length_at_target({32: 0.2, 64: 0.8, 128: 0.99}, 0.98) == 128


def test_invalid_binomial_counts_are_rejected() -> None:
    with pytest.raises(ValueError):
        clopper_pearson(2, 1)
