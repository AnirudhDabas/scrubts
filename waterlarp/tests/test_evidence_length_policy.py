import inspect
from dataclasses import replace

from waterlarp.calibration.thresholds import (
    ThresholdContext,
    calibrate_threshold,
    select_threshold,
)
from waterlarp.config import Comparator, ThresholdSemantics


def calibrated(length: int):
    context = replace(ThresholdContext.fixture(Comparator.STRICT_GREATER), scored_unit_count=length)
    return calibrate_threshold([0.0] * 100, 0.01, context=context, comparator=context.comparator)


def test_exact_boundary_below_between_and_above_support_are_explicit() -> None:
    thresholds = [calibrated(10), calibrated(20)]
    base = thresholds[0].context
    assert select_threshold(thresholds, base).status == "SELECTED"
    assert select_threshold(thresholds, replace(base, scored_unit_count=5)).status == "UNSUPPORTED"
    between = select_threshold(thresholds, replace(base, scored_unit_count=15))
    assert between.status == "UNRESOLVED"
    assert "between" in between.reason
    above = select_threshold(thresholds, replace(base, scored_unit_count=25))
    assert above.status == "UNRESOLVED"
    assert "exceeds" in above.reason
    zero = select_threshold(thresholds, replace(base, scored_unit_count=0))
    assert zero.status == "UNSUPPORTED"


def test_deletion_crossing_exact_evidence_length_uses_only_post_edit_units() -> None:
    clean = calibrated(29)
    conditioned_context = replace(
        clean.context,
        threshold_semantics=ThresholdSemantics.OPERATION_CONDITIONED_THRESHOLD,
        regime="operation_conditioned",
        operation="random_deletion",
        operation_strength=0.1,
        scored_unit_count=26,
    )
    conditioned = calibrate_threshold(
        [0.0] * 100,
        0.01,
        context=conditioned_context,
        comparator=conditioned_context.comparator,
    )
    assert select_threshold([conditioned], conditioned_context).status == "SELECTED"
    fixed_request = replace(clean.context, scored_unit_count=26)
    assert select_threshold([clean], fixed_request).status == "UNSUPPORTED"


def test_threshold_selector_has_no_original_or_nominal_length_parameter() -> None:
    parameters = inspect.signature(select_threshold).parameters
    assert "original_length" not in parameters
    assert "nominal_length" not in parameters
