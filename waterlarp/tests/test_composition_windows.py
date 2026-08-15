from dataclasses import replace

import pytest

from waterlarp.calibration.thresholds import ThresholdContext, decisions
from waterlarp.composition.spans import compose_tokens
from waterlarp.composition.windows import (
    WindowSearchSpecification,
    calibrate_window_search,
    fixed_window_max,
)
from waterlarp.config import Comparator, CoordinateSystem
from waterlarp.metrics.confidence import clopper_pearson
from waterlarp.metrics.localization import localization_report, span_iou


def test_composition_records_exact_boundaries_and_separated_spans() -> None:
    document = compose_tokens(range(100, 200), range(100), 0.25, separated_segments=True, seed=9)
    assert sum(segment.end - segment.start for segment in document.segments) == len(
        document.token_ids
    )
    assert len([segment for segment in document.segments if segment.marked]) == 2
    assert 0 < document.realized_marked_fraction < 1


def test_window_max_calibration_and_test_use_identical_search_identity() -> None:
    specification = WindowSearchSpecification(
        "TOKEN", 4, 2, 1, "all complete windows", "fixture evidence policy"
    )

    def score(values: list[int]) -> float:
        return float(sum(values))

    result = fixed_window_max([0, 0, 9, 9, 0], score, window_size=2, stride=1)
    assert (result.maximum.start, result.maximum.end, result.maximum.score) == (2, 4, 18.0)
    context = replace(
        ThresholdContext.fixture(Comparator.GREATER_OR_EQUAL),
        regime="window_search",
        search_spec_sha256=specification.sha256,
    )
    calibration = calibrate_window_search(
        [[0, 0, 0, value] for value in range(100)],
        score,
        window_size=specification.window_size,
        stride=specification.stride,
        target_document_fpr=0.01,
        context=context,
        comparator=context.comparator,
    )
    assert calibration.context.search_spec_sha256 == specification.sha256
    assert calibration.negative_count == 100
    assert calibration.false_positive_count == 1
    zero_fp = decisions([fixed_window_max([0, 0, 0, 0], score, 2, 1).maximum.score], calibration)
    one_fp = decisions([fixed_window_max([0, 0, 0, 100], score, 2, 1).maximum.score], calibration)
    assert sum(zero_fp) == 0
    assert sum(one_fp) == 1
    assert clopper_pearson(sum(one_fp), len(one_fp)).successes == 1


@pytest.mark.parametrize(
    ("predicted", "truths", "expected_iou", "expected_overlap"),
    (
        ((10, 20), ((10, 20),), 1.0, 10),
        ((10, 20), ((15, 25),), 1 / 3, 5),
        ((0, 5), ((10, 20),), 0.0, 0),
        ((0, 10), ((0, 5), (8, 12)), 7 / 12, 7),
        ((0, 1), ((1, 2),), 0.0, 0),
    ),
)
def test_typed_token_localization_vectors(
    predicted: tuple[int, int],
    truths: tuple[tuple[int, int], ...],
    expected_iou: float,
    expected_overlap: int,
) -> None:
    report = localization_report(predicted, truths)
    assert report.coordinate_system is CoordinateSystem.TOKEN
    assert report.iou == pytest.approx(expected_iou)
    assert report.overlap_token_count == expected_overlap


def test_localization_rejects_non_token_or_ambiguous_boundaries() -> None:
    with pytest.raises(ValueError, match="TOKEN"):
        localization_report((0, 1), ((0, 1),), coordinate_system="CHARACTER")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="overlap"):
        localization_report((0, 1), ((0, 2), (1, 3)))
    assert span_iou((10, 20), (15, 25)) == 1 / 3
