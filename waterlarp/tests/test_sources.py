import socket
from pathlib import Path

import pytest

from waterlarp.authority import AUTHORITY_RECORDS
from waterlarp.authority_sources import load_source_ledger


def test_repository_source_ledger_has_unique_ids() -> None:
    package_root = Path(__file__).resolve().parents[1]
    ledger = load_source_ledger(package_root.parent / "research/sources.yaml")
    assert len(ledger["sources"]) == len({source["id"] for source in ledger["sources"]})
    source_ids = {source["id"] for source in ledger["sources"]}
    assert all(
        set(record.authority_source_ids) <= source_ids for record in AUTHORITY_RECORDS.values()
    )


def test_source_ledger_validation_is_offline_and_local(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fail_network(*_args: object, **_kwargs: object) -> None:
        raise AssertionError("source-ledger validation attempted network access")

    monkeypatch.setattr(socket, "create_connection", fail_network)
    package_root = Path(__file__).resolve().parents[1]
    ledger = load_source_ledger(package_root.parent / "research/sources.yaml")
    assert ledger["sources"]


def test_anthropic_primary_sources_freeze_family_without_provider_parity() -> None:
    package_root = Path(__file__).resolve().parents[1]
    ledger = load_source_ledger(package_root.parent / "research/sources.yaml")
    sources = {source["id"]: source for source in ledger["sources"]}
    technical = sources["anthropic-claude-text-watermark"]
    assert technical["organization"] == "Anthropic"
    assert technical["publication_date"].isoformat() == "2026-08-14"
    assert technical["claim_classification"] == "vendor-reported"
    assert any(
        "version of the SynthID-Text approach" in statement
        for statement in technical["authoritative_for"]
    )
    assert any("does not establish deployment parity" in item for item in technical["limitations"])
    assert "Claude deployment" in sources["synthid-text"]["not_authoritative_for"][1]
    assert sources["eu-ai-transparency-code-2026"]["integration"] == "policy-context-only"


def test_duplicate_source_ids_fail(tmp_path: Path) -> None:
    path = tmp_path / "sources.yaml"
    path.write_text("sources:\n- id: x\n- id: x\n")
    with pytest.raises(ValueError, match="unique"):
        load_source_ledger(path)


def test_post_review_citation_slots_are_exact_and_non_runnable() -> None:
    package_root = Path(__file__).resolve().parents[1]
    ledger = load_source_ledger(package_root.parent / "research/sources.yaml")
    sources = {source["id"]: source for source in ledger["sources"]}
    under_fire = sources["waterpark-under-fire"]
    assert under_fire["title"] == (
        "Watermark under Fire: A Robustness Evaluation of LLM Watermarking"
    )
    assert under_fire["acl_anthology_id"] == "2025.findings-emnlp.1148"
    assert under_fire["doi"] == "10.18653/v1/2025.findings-emnlp.1148"
    assert under_fire["revision"] == "76b66dfa604075c9c79be71dcaebb5afe652d882"
    assert "no repository-wide license" in under_fire["license"]
    assert under_fire["integration"] == "citation-only"
    assert under_fire["runnable"] is False

    sandcastles = sources["sandcastles-in-storm"]
    assert sandcastles["title"] == (
        "Sandcastles in the Storm: Revisiting the (Im)possibility of Strong Watermarking"
    )
    assert sandcastles["acl_anthology_id"] == "2025.acl-long.1436"
    assert sandcastles["doi"] == "10.18653/v1/2025.acl-long.1436"
    assert sandcastles["integration"] == "citation-only"
    assert sandcastles["runnable"] is False

    smoothing = sources["confidence-smoothing-attack"]
    assert smoothing["title"] == "Watermark Smoothing Attacks against Language Models"
    assert smoothing["acl_anthology_id"] == "2025.findings-emnlp.264"
    assert smoothing["doi"] == "10.18653/v1/2025.findings-emnlp.264"
    assert smoothing["revision"] == "5acda5f1f27ddebe758d051537b0a59982f89b22"
    assert smoothing["license"].startswith("Apache-2.0")
    assert smoothing["integration"] == "citation-only"
    assert smoothing["runnable"] is False
