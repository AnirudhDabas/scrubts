from dataclasses import replace

import pytest
import torch

from waterlarp.adapters.base import DetectionScore
from waterlarp.calibration.thresholds import (
    ThresholdContext,
    calibrate_threshold,
    select_threshold,
    selected_decision,
)
from waterlarp.config import Comparator
from waterlarp.evidence import canonical_detector_evidence, rescore_canonical_record


class SumAdapter:
    def metadata(self) -> dict[str, object]:
        return {"name": "sum", "revision": "1", "key": "fixture"}

    def score_token_ids(self, token_ids: list[int], tokenizer: object) -> DetectionScore:
        score = float(sum(token_ids))
        return DetectionScore(score, "sum", len(token_ids), score / 100, {"sum": score})


def fixture_record(*, window: bool = False) -> tuple[dict[str, object], list[dict[str, object]]]:
    adapter = SumAdapter()
    token_ids = [0, 1, 9, 9, 0] if window else [1, 2, 3]
    if window:
        score = adapter.score_token_ids([9, 9], object())
        procedure = {
            "coordinate_system": "TOKEN",
            "document_token_count": 5,
            "window_size": 2,
            "stride": 1,
            "valid_window_policy": "all",
            "detector_evidence_policy": "fixture",
            "search_spec_id": "wlrx1-fixture",
            "search_spec_sha256": "a" * 64,
            "best_window": {"start": 2, "end": 4, "score": 18.0},
        }
        context = replace(
            ThresholdContext.fixture(Comparator.STRICT_GREATER),
            scored_unit_count=2,
            regime="window_search",
            search_spec_sha256="a" * 64,
        )
    else:
        score = adapter.score_token_ids(token_ids, object())
        procedure = None
        context = replace(ThresholdContext.fixture(Comparator.STRICT_GREATER), scored_unit_count=3)
    threshold = calibrate_threshold(
        [0.0] * 100, 0.01, context=context, comparator=context.comparator
    )
    selection = select_threshold([threshold], context)
    decision, status = selected_decision(score.score, selection)
    evidence = canonical_detector_evidence(
        detector_input_token_ids=token_ids,
        tokenizer_identity={"revision": "fixture"},
        detector_metadata=adapter.metadata(),
        key_provenance="fixture",
        score=score,
        threshold_request=context,
        selection=selection,
        decision=decision,
        decision_status=status,
        procedure=procedure,
    )
    return (
        {
            "scheme": "fixture",
            "decision": decision,
            "decision_status": status,
            "detector_evidence": evidence,
        },
        [threshold.to_dict()],
    )


@pytest.mark.parametrize("window", (False, True))
def test_canonical_evidence_rescores_without_generation_checkpoint(window: bool) -> None:
    record, thresholds = fixture_record(window=window)
    result = rescore_canonical_record(
        record,
        adapters={"fixture": SumAdapter()},
        tokenizer=object(),
        threshold_records=thresholds,
    )
    assert result.decision is True
    assert result.threshold_id == thresholds[0]["threshold_id"]


def test_changed_detector_input_is_detected() -> None:
    record, thresholds = fixture_record()
    record["detector_evidence"]["detector_input_token_ids"][0] = 99  # type: ignore[index]
    with pytest.raises(ValueError, match="raw score"):
        rescore_canonical_record(
            record,
            adapters={"fixture": SumAdapter()},
            tokenizer=object(),
            threshold_records=thresholds,
        )


def test_detector_native_tensor_evidence_is_promoted_as_json_values() -> None:
    context = replace(ThresholdContext.fixture(Comparator.STRICT_GREATER), scored_unit_count=1)
    threshold = calibrate_threshold(
        [0.0] * 100, 0.01, context=context, comparator=context.comparator
    )
    selection = select_threshold([threshold], context)
    score = DetectionScore(
        1.0,
        "fixture",
        1,
        None,
        {"scalar": torch.tensor(1.25), "vector": torch.tensor([1, 2])},
    )
    evidence = canonical_detector_evidence(
        detector_input_token_ids=[1],
        tokenizer_identity={"revision": "fixture"},
        detector_metadata={"revision": "fixture"},
        key_provenance="fixture",
        score=score,
        threshold_request=context,
        selection=selection,
        decision=True,
        decision_status="DETECTED",
    )
    assert evidence["raw_detector_evidence"] == {"scalar": 1.25, "vector": [1, 2]}
