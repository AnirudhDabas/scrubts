"""Deterministic aggregation from sample records, never handwritten tables."""

from __future__ import annotations

import hashlib
from collections import defaultdict
from collections.abc import Iterable, Mapping
from statistics import mean, median
from typing import Any

import numpy as np

from waterlarp.manifests import canonical_json_bytes
from waterlarp.metrics.confidence import clopper_pearson
from waterlarp.metrics.detection import detection_rates

GROUP_FIELDS = (
    "scheme",
    "mode",
    "task",
    "entropy_bucket",
    "nominal_generation_length",
    "evidence_length",
    "threshold_semantics",
    "threshold_id",
    "evaluation_kind",
    "transform",
    "transform_strength",
    "marked_fraction",
    "layout",
    "search_spec_id",
)


def _quantiles(values: list[float]) -> dict[str, float]:
    return {str(q): float(np.quantile(values, q)) for q in (0.05, 0.25, 0.5, 0.75, 0.95)}


def aggregate_records(
    records: Iterable[Mapping[str, Any]], target_fpr: float
) -> list[dict[str, Any]]:
    groups: dict[tuple[Any, ...], list[Mapping[str, Any]]] = defaultdict(list)
    for record in records:
        groups[tuple(record.get(field) for field in GROUP_FIELDS)].append(record)
    results: list[dict[str, Any]] = []
    for key, members in sorted(
        groups.items(), key=lambda item: tuple(str(value) for value in item[0])
    ):
        positive = [
            bool(member["decision"])
            for member in members
            if member["label"] == "watermarked" and member.get("decision") is not None
        ]
        negative = [
            bool(member["decision"])
            for member in members
            if member["label"] == "unwatermarked" and member.get("decision") is not None
        ]
        attempted_negative = [member for member in members if member["label"] == "unwatermarked"]
        unresolved_negative = [
            member for member in attempted_negative if member.get("decision") is None
        ]
        unresolved = sum(member.get("decision") is None for member in members)
        rates = detection_rates(positive, negative, target_fpr) if positive and negative else None
        fpr = clopper_pearson(sum(negative), len(negative)) if negative else None
        scores = [float(member["score"]) for member in members]
        quality_values = [
            float(value)
            for member in members
            for value in member.get("quality", {}).values()
            if isinstance(value, (int, float)) and not isinstance(value, bool)
        ]
        group = dict(zip(GROUP_FIELDS, key, strict=True))
        identity = hashlib.sha256(canonical_json_bytes(group)).hexdigest()[:24]
        results.append(
            {
                "schema_version": "2.0.0",
                "aggregate_id": f"wlra1-{identity}",
                "group": group,
                "run_ids": sorted({str(member["run_id"]) for member in members}),
                "manifest_paths": sorted({str(member["manifest_path"]) for member in members}),
                "source_authority_ids": sorted(
                    {source for member in members for source in member["source_authority_ids"]}
                ),
                "counts": {
                    "N": len(members),
                    "positive_N": len(positive),
                    "negative_N": len(negative),
                    "held_out_negative_N": len(attempted_negative),
                    "unresolved_N": unresolved,
                },
                "detection": None if rates is None else rates,
                "held_out_fpr": None
                if not attempted_negative
                else {
                    "false_positive_count": sum(negative),
                    "attempted_negative_count": len(attempted_negative),
                    "negative_count": len(negative),
                    "unresolved_negative_count": len(unresolved_negative),
                    "empirical_fpr": (
                        fpr.point_estimate if fpr is not None and not unresolved_negative else None
                    ),
                    "confidence_interval_95": (
                        {
                            "method": fpr.interval_method,
                            "lower": fpr.lower,
                            "upper": fpr.upper,
                        }
                        if fpr is not None and not unresolved_negative
                        else None
                    ),
                    "resolution_status": (
                        "UNSUPPORTED"
                        if unresolved_negative
                        and all(
                            member.get("decision_status") == "UNSUPPORTED"
                            for member in unresolved_negative
                        )
                        else "UNRESOLVED"
                        if unresolved_negative or len(negative) * target_fpr < 1
                        else "RESOLVED"
                    ),
                    "target_fpr": target_fpr,
                },
                "scores": {
                    "mean": mean(scores),
                    "median": median(scores),
                    "quantiles": _quantiles(scores),
                },
                "quality": None
                if not quality_values
                else {
                    "mean": mean(quality_values),
                    "median": median(quality_values),
                    "quantiles": _quantiles(quality_values),
                },
            }
        )
    return results


def subgroup_false_positives(
    records: Iterable[Mapping[str, Any]], subgroup: str
) -> dict[str, dict[str, int | float]]:
    groups: dict[str, list[bool]] = defaultdict(list)
    for record in records:
        if record["label"] == "unwatermarked" and record.get("decision") is not None:
            groups[str(record.get(subgroup, "UNSPECIFIED"))].append(bool(record["decision"]))
    return {
        group: {
            "false_positive_count": sum(values),
            "negative_count": len(values),
            "fpr": sum(values) / len(values),
        }
        for group, values in sorted(groups.items())
    }
