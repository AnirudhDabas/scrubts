from waterlarp.metrics.aggregate import aggregate_records, subgroup_false_positives


def record(
    label: str,
    decision: bool | None,
    task: str,
    score: float,
    *,
    strength: float = 0.1,
) -> dict[str, object]:
    return {
        "run_id": "wlrp1-fixture",
        "manifest_path": "manifest.json",
        "source_authority_ids": ["kgw"],
        "scheme": "reference.kgw",
        "mode": "authoritative_default",
        "task": task,
        "entropy_bucket": "high",
        "nominal_generation_length": 64,
        "evidence_length": 58,
        "threshold_semantics": "operation_conditioned_threshold",
        "threshold_id": f"threshold-{task}-{strength}",
        "evaluation_kind": "operation_held_out",
        "transform": "random_deletion",
        "transform_strength": strength,
        "marked_fraction": 1.0,
        "layout": None,
        "search_spec_id": None,
        "label": label,
        "decision": decision,
        "score": score,
        "quality": {"edit_distance": 0.1},
    }


def test_aggregation_derives_zero_held_out_false_positives_and_exact_interval() -> None:
    records = [
        record("watermarked", True, "c4", 4.0),
        record("unwatermarked", False, "c4", 0.1),
    ]
    aggregate = aggregate_records(records, 0.01)[0]
    assert aggregate["counts"] == {
        "N": 2,
        "positive_N": 1,
        "negative_N": 1,
        "held_out_negative_N": 1,
        "unresolved_N": 0,
    }
    assert aggregate["detection"].status == "UNRESOLVED"
    assert aggregate["held_out_fpr"]["false_positive_count"] == 0
    assert aggregate["held_out_fpr"]["confidence_interval_95"]["upper"] == 0.975
    assert subgroup_false_positives(records, "task")["c4"]["fpr"] == 0


def test_one_held_out_false_positive_is_real_and_task_strength_isolated() -> None:
    records = [
        record("watermarked", True, "c4", 4.0),
        record("unwatermarked", True, "c4", 3.0),
        record("watermarked", True, "mbpp", 4.0),
        record("unwatermarked", False, "mbpp", 0.0),
        record("watermarked", True, "c4", 4.0, strength=0.2),
        record("unwatermarked", False, "c4", 0.0, strength=0.2),
    ]
    aggregates = aggregate_records(records, 0.01)
    assert len(aggregates) == 3
    keyed = {
        (item["group"]["task"], item["group"]["transform_strength"]): item for item in aggregates
    }
    assert keyed[("c4", 0.1)]["held_out_fpr"]["false_positive_count"] == 1
    assert keyed[("mbpp", 0.1)]["held_out_fpr"]["false_positive_count"] == 0
    assert keyed[("c4", 0.2)]["held_out_fpr"]["false_positive_count"] == 0


def test_unresolved_decisions_do_not_become_false_negatives_or_negatives() -> None:
    records = [record("watermarked", None, "c4", 4.0), record("unwatermarked", None, "c4", 0.1)]
    aggregate = aggregate_records(records, 0.01)[0]
    assert aggregate["counts"]["unresolved_N"] == 2
    assert aggregate["detection"] is None
    assert aggregate["held_out_fpr"]["attempted_negative_count"] == 1
    assert aggregate["held_out_fpr"]["negative_count"] == 0
    assert aggregate["held_out_fpr"]["empirical_fpr"] is None
    assert aggregate["held_out_fpr"]["confidence_interval_95"] is None
    assert aggregate["held_out_fpr"]["resolution_status"] == "UNRESOLVED"


def test_held_out_fpr_does_not_require_a_resolved_positive_in_same_length_group() -> None:
    aggregate = aggregate_records([record("unwatermarked", True, "c4", 3.0)], 0.01)[0]
    assert aggregate["detection"] is None
    assert aggregate["held_out_fpr"]["false_positive_count"] == 1
    assert aggregate["held_out_fpr"]["negative_count"] == 1
