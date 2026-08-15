#!/usr/bin/env python3
"""Offline verification for the Mega D-A Claude claims audit artifact."""

from __future__ import annotations

from datetime import datetime, timedelta
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
AUDIT_ROOT = ROOT / "research" / "claude-watermark-claims"
CLAIMS_PATH = AUDIT_ROOT / "claims.json"
SOURCES_PATH = AUDIT_ROOT / "sources.yaml"
FIXTURE_METADATA_PATH = AUDIT_ROOT / "fixtures" / "controlled-u200b.fixture.json"
RUN_PATH = AUDIT_ROOT / "evidence" / "scrub-run.json"
README_PATH = AUDIT_ROOT / "README.md"
GIT_REVISION = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
EXPECTED_CLAIM_SEMANTICS = {
    "claudewatermark-unicode-provider-detection": {
        "classification": "unsupported_provider_inference",
        "required_authority": "provider_gated",
        "available_authority": "public_observation",
    },
    "gptcleanup-supported-hidden-unicode": {
        "classification": "accurately_limited",
        "required_authority": "public_observation",
        "available_authority": "public_observation",
    },
    "overchat-claude-detection-100-percent": {
        "classification": "unsupported_provider_inference",
        "required_authority": "public_verification",
        "available_authority": "none",
    },
    "overchat-private-detector-caveat": {
        "classification": "accurately_limited",
        "required_authority": "provider_gated",
        "available_authority": "public_observation",
    },
    "google-synthid-open-source-mechanism": {
        "classification": "mechanism_demo_not_provider_detector",
        "required_authority": "public_reference",
        "available_authority": "public_reference",
    },
    "synthid-text-reference-implementation": {
        "classification": "mechanism_demo_not_provider_detector",
        "required_authority": "public_reference",
        "available_authority": "public_reference",
    },
    "watermarks-remover-vendor-detector-caveat": {
        "classification": "accurately_limited",
        "required_authority": "provider_gated",
        "available_authority": "public_reference",
    },
}


def waterlarp_python() -> Path:
    candidates = (
        ROOT / "waterlarp" / ".venv" / "Scripts" / "python.exe",
        ROOT / "waterlarp" / ".venv" / "bin" / "python",
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise RuntimeError(
        "jsonschema/PyYAML are unavailable and the WaterLARP validation environment is missing"
    )


def ensure_validation_runtime() -> None:
    try:
        import jsonschema  # noqa: F401
        import yaml  # noqa: F401
    except ImportError:
        interpreter = waterlarp_python()
        if Path(sys.executable).resolve() == interpreter.resolve():
            raise
        completed = subprocess.run(
            [str(interpreter), str(Path(__file__).resolve()), *sys.argv[1:]],
            cwd=ROOT,
        )
        raise SystemExit(completed.returncode)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path.relative_to(ROOT)} must contain a JSON object")
    return value


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_utc(value: str, label: str) -> datetime:
    if not value.endswith("Z"):
        raise ValueError(f"{label} is not a UTC Z timestamp: {value!r}")
    parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    if parsed.utcoffset() != timedelta(0):
        raise ValueError(f"{label} is not UTC: {value!r}")
    return parsed


def repository_path(relative: str) -> Path:
    path = ROOT / relative
    resolved = path.resolve()
    if not resolved.is_relative_to(ROOT.resolve()):
        raise ValueError(f"external artifact path: {relative}")
    return path


def validate_claim_semantics(claims: dict[str, dict[str, Any]]) -> None:
    actual_ids = set(claims)
    expected_ids = set(EXPECTED_CLAIM_SEMANTICS)
    if actual_ids != expected_ids:
        raise ValueError(
            "claim semantic identity mismatch: "
            f"missing={sorted(expected_ids - actual_ids)} "
            f"extra={sorted(actual_ids - expected_ids)}"
        )

    for claim_id, expected in EXPECTED_CLAIM_SEMANTICS.items():
        actual = {
            field: claims[claim_id][field]
            for field in ("classification", "required_authority", "available_authority")
        }
        if actual != expected:
            raise ValueError(
                f"claim {claim_id} semantic identity mismatch: "
                f"expected={expected} actual={actual}"
            )


def validate_claims() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    from jsonschema import Draft202012Validator, FormatChecker

    claims_document = load_json(CLAIMS_PATH)
    schema = load_json(ROOT / "schemas" / "claude-watermark-claim-audit-0.1.schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema, format_checker=FormatChecker()).validate(claims_document)
    parse_utc(claims_document["captured_at_utc"], "claims captured_at_utc")

    classification_values = set(schema["$defs"]["classification"]["enum"])
    authority_values = set(schema["$defs"]["authority_class"]["enum"])
    if set(claims_document["classification_definitions"]) != classification_values:
        raise ValueError("classification definitions do not cover the schema enum exactly")
    if set(claims_document["authority_class_definitions"]) != authority_values:
        raise ValueError("authority definitions do not cover the schema enum exactly")

    claims: dict[str, dict[str, Any]] = {}
    for claim in claims_document["claims"]:
        claim_id = claim["claim_id"]
        if claim_id in claims:
            raise ValueError(f"duplicate claim ID: {claim_id}")
        parse_utc(claim["captured_at_utc"], f"claim {claim_id} captured_at_utc")
        claims[claim_id] = claim
    validate_claim_semantics(claims)
    return claims_document, claims


def validate_sources(claims: dict[str, dict[str, Any]]) -> dict[str, dict[str, Any]]:
    import yaml

    document = yaml.safe_load(SOURCES_PATH.read_text(encoding="utf-8"))
    if set(document) != {"schema_version", "captured_at_utc", "sources"}:
        raise ValueError("sources.yaml has unexpected top-level fields")
    if document["schema_version"] != "0.1":
        raise ValueError("unsupported sources schema version")
    parse_utc(document["captured_at_utc"], "sources captured_at_utc")

    required = {
        "source_id",
        "title",
        "publisher",
        "url",
        "source_type",
        "captured_at_utc",
        "authority_class",
        "integration_mode",
        "revision",
        "capture",
        "relevant_claims",
        "relevant_excerpt",
        "limitations",
        "license",
    }
    sources: dict[str, dict[str, Any]] = {}
    for source in document["sources"]:
        missing = required - set(source)
        if missing:
            raise ValueError(f"source record is missing fields: {sorted(missing)}")
        source_id = source["source_id"]
        if source_id in sources:
            raise ValueError(f"duplicate source ID: {source_id}")
        parse_utc(source["captured_at_utc"], f"source {source_id} captured_at_utc")
        if source["source_type"] == "repository" and not GIT_REVISION.fullmatch(
            source["revision"]
        ):
            raise ValueError(f"source {source_id} has an invalid pinned revision")
        capture = source["capture"]
        if not isinstance(capture.get("byte_length"), int) or capture["byte_length"] <= 0:
            raise ValueError(f"source {source_id} has an invalid captured byte length")
        if not SHA256.fullmatch(capture.get("sha256", "")):
            raise ValueError(f"source {source_id} has an invalid capture SHA-256")
        unknown_claims = set(source["relevant_claims"]) - set(claims)
        if unknown_claims:
            raise ValueError(f"source {source_id} has unknown claims: {sorted(unknown_claims)}")
        sources[source_id] = source

    for claim_id, claim in claims.items():
        if claim["source_id"] not in sources:
            raise ValueError(f"claim {claim_id} has an unknown source_id")
        unknown_support = set(claim["supporting_source_ids"]) - set(sources)
        if unknown_support:
            raise ValueError(
                f"claim {claim_id} has unknown supporting sources: {sorted(unknown_support)}"
            )
        if claim_id not in sources[claim["source_id"]]["relevant_claims"]:
            raise ValueError(f"claim {claim_id} is absent from its source's relevant_claims")

    anthropic_findings = sources["anthropic-claude-text-watermark"].get(
        "authoritative_findings", {}
    )
    if set(anthropic_findings) != {
        "mechanism_family",
        "carrier",
        "detector_authority",
        "api_status",
        "inference_boundary",
        "weak_signal_conditions",
        "rewriting",
        "file_provenance",
    } or not all(isinstance(value, str) and value for value in anthropic_findings.values()):
        raise ValueError("Anthropic source intake does not contain the eight required findings")
    google_findings = sources["google-synthid-safeguards"].get(
        "authoritative_findings", {}
    )
    if set(google_findings) != {
        "public_mechanism",
        "configuration_authority",
        "verification_models",
    } or not all(isinstance(value, str) and value for value in google_findings.values()):
        raise ValueError("SynthID source intake does not contain the required authority findings")

    sample_sources = {claim["source_id"] for claim in claims.values()}
    if not 5 <= len(sample_sources) <= 8:
        raise ValueError(f"convenience sample size is outside 5-8: {len(sample_sources)}")
    return sources


def validate_fixture() -> tuple[dict[str, Any], bytes]:
    metadata = load_json(FIXTURE_METADATA_PATH)
    fixture_path = repository_path(metadata["fixture_file"])
    visible_path = repository_path(metadata["visible_rendering_file"])
    generator_path = repository_path(metadata["generator_file"])
    fixture = fixture_path.read_bytes()
    visible = visible_path.read_bytes()
    injected = bytes.fromhex(metadata["injected_utf8_hex"])
    byte_offset = metadata["byte_offset_zero_based"]

    if len(fixture) != metadata["fixture_byte_length"] or sha256_file(fixture_path) != metadata[
        "fixture_sha256"
    ]:
        raise ValueError("controlled fixture byte identity does not match metadata")
    if len(visible) != metadata["visible_byte_length"] or sha256_file(visible_path) != metadata[
        "visible_sha256"
    ]:
        raise ValueError("visible rendering byte identity does not match metadata")
    if sha256_file(generator_path) != metadata["generator_sha256"]:
        raise ValueError("fixture generator identity does not match metadata")
    if fixture.hex() != metadata["fixture_hex"]:
        raise ValueError("fixture hex does not match metadata")
    if fixture.count(injected) != metadata["occurrence_count"] or fixture.count(injected) != 1:
        raise ValueError("fixture does not contain exactly one injected code point")
    if fixture[byte_offset : byte_offset + len(injected)] != injected:
        raise ValueError("injected bytes are not at the documented byte offset")
    if fixture[:byte_offset] + fixture[byte_offset + len(injected) :] != visible:
        raise ValueError("removing the documented injected bytes does not recover visible rendering")
    decoded = fixture.decode("utf-8")
    scalar_offset = metadata["scalar_offset_zero_based"]
    if decoded[scalar_offset] != "\u200b" or decoded.count("\u200b") != 1:
        raise ValueError("U+200B is not at the documented scalar offset")
    if visible.decode("utf-8") != metadata["visible_rendering"]:
        raise ValueError("recorded visible rendering does not match companion bytes")
    return metadata, fixture


def index_report_findings(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    mechanism_ids = [finding["mechanism"]["id"] for finding in report["findings"]]
    seen: set[str] = set()
    duplicate_ids: set[str] = set()
    for mechanism_id in mechanism_ids:
        if mechanism_id in seen:
            duplicate_ids.add(mechanism_id)
        seen.add(mechanism_id)
    if duplicate_ids:
        raise ValueError(f"duplicate report mechanism IDs: {sorted(duplicate_ids)}")
    return {finding["mechanism"]["id"]: finding for finding in report["findings"]}


def validate_evidence(metadata: dict[str, Any], fixture: bytes) -> dict[str, Any]:
    from jsonschema import Draft202012Validator

    run = load_json(RUN_PATH)
    parse_utc(run["captured_at_utc"], "evidence captured_at_utc")
    if not GIT_REVISION.fullmatch(run["project_revision"]):
        raise ValueError("evidence project revision is not a full Git revision")
    if run["input"]["fixture_id"] != metadata["fixture_id"]:
        raise ValueError("evidence fixture ID does not match fixture metadata")
    if run["input"]["byte_length"] != len(fixture) or run["input"]["sha256"] != hashlib.sha256(
        fixture
    ).hexdigest():
        raise ValueError("evidence input identity does not match fixture bytes")

    expected_base = [
        "cargo",
        "run",
        "--offline",
        "--quiet",
        "-p",
        "scrub",
        "--",
        "inspect",
        run["input"]["path"],
    ]
    if run["machine_command"] != [*expected_base, "--json"]:
        raise ValueError("stored machine command is not the expected real CLI invocation")
    if run["explain_command"] != [*expected_base, "--explain"]:
        raise ValueError("stored explain command is not the expected real CLI invocation")

    output_paths: dict[str, Path] = {}
    for kind, record in run["outputs"].items():
        path = repository_path(record["path"])
        if path.stat().st_size != record["byte_length"] or sha256_file(path) != record["sha256"]:
            raise ValueError(f"stored {kind} output identity does not match run manifest")
        output_paths[kind] = path

    report = load_json(output_paths["machine"])
    report_schema = load_json(ROOT / "schemas" / "report-0.2.schema.json")
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    if report["artifact"] != {
        "path": Path(run["input"]["path"]).name,
        "byte_length": run["input"]["byte_length"],
        "content_sha256": run["input"]["sha256"],
    }:
        raise ValueError("stored report does not identify the exact fixture")

    findings = index_report_findings(report)
    unicode_finding = findings["unicode.default_ignorable_code_point"]
    provider_finding = findings["anthropic.embedded_text_watermark"]
    if unicode_finding["status"] != "present":
        raise ValueError("controlled fixture Unicode result is not PRESENT")
    evidence = {item["name"]: item["value"] for item in unicode_finding["evidence"]}
    expected_location = [
        {
            "code_point": metadata["injected_code_point"],
            "byte_offset": metadata["byte_offset_zero_based"],
            "scalar_offset": metadata["scalar_offset_zero_based"],
        }
    ]
    if json.loads(evidence["locations"]) != expected_location or evidence[
        "total_occurrence_count"
    ] != "1":
        raise ValueError("stored Unicode evidence does not identify the controlled injection")
    if provider_finding["status"] != "unknown":
        raise ValueError("controlled fixture Anthropic provider result is not UNKNOWN")
    if "provider_detector_unavailable" not in provider_finding["trace"]["supports"]:
        raise ValueError("provider result does not retain detector-unavailable support")
    forbidden = set(provider_finding["trace"]["does_not_support"])
    if not {"claude_watermark_present", "claude_watermark_absent", "claude_provider_parity"} <= forbidden:
        raise ValueError("provider result does not forbid required unsupported inferences")

    explain = output_paths["explain"].read_text(encoding="utf-8")
    required_explain = (
        "Unicode  PRESENT      Default_Ignorable_Code_Point",
        "Claude   UNKNOWN      embedded text watermark",
        'locations=[{"code_point":"U+200B","byte_offset":4,"scalar_offset":4}]',
        "related reference reference.synthid_text (related family; not deployment parity)",
    )
    for excerpt in required_explain:
        if excerpt not in explain:
            raise ValueError(f"stored explain output is missing: {excerpt}")
    if run["observed_summary"] != {
        "unicode_default_ignorable_code_point": unicode_finding["status"],
        "anthropic_embedded_text_watermark": provider_finding["status"],
    }:
        raise ValueError("run summary has drifted from stored machine output")
    return run


def validate_summary_table(claims: dict[str, dict[str, Any]], run: dict[str, Any]) -> None:
    readme = README_PATH.read_text(encoding="utf-8")
    start = readme.index("<!-- claim-summary:start -->")
    end = readme.index("<!-- claim-summary:end -->", start)
    table = readme[start:end].splitlines()
    rows = [line for line in table if line.startswith("| `")]
    seen: set[str] = set()
    for row in rows:
        cells = [cell.strip().strip("`") for cell in row.strip().strip("|").split("|")]
        if len(cells) != 5:
            raise ValueError(f"malformed README summary row: {row}")
        claim_id, source_id, observable, authority, classification = cells
        if claim_id not in claims:
            raise ValueError(f"README summary contains an unknown claim: {claim_id}")
        if claim_id in seen:
            raise ValueError(f"README summary duplicates claim: {claim_id}")
        claim = claims[claim_id]
        if source_id != claim["source_id"] or classification != claim["classification"]:
            raise ValueError(f"README summary source/classification drift for {claim_id}")
        expected_authority = f'{claim["required_authority"]} / {claim["available_authority"]}'
        if authority != expected_authority or not observable:
            raise ValueError(f"README summary authority/observable drift for {claim_id}")
        seen.add(claim_id)
    if seen != set(claims):
        raise ValueError(f"README summary claim set mismatch: missing={sorted(set(claims) - seen)}")
    machine_command = " ".join(run["machine_command"])
    if machine_command not in readme:
        raise ValueError("README does not contain the exact stored machine command")


def main() -> int:
    ensure_validation_runtime()
    _, claims = validate_claims()
    sources = validate_sources(claims)
    fixture_metadata, fixture = validate_fixture()
    run = validate_evidence(fixture_metadata, fixture)
    validate_summary_table(claims, run)
    sample_sources = {claim["source_id"] for claim in claims.values()}
    print("claude watermark claim audit: PASS")
    print(f"claims={len(claims)} sample_sources={len(sample_sources)} source_records={len(sources)}")
    print(f"fixture_sha256={fixture_metadata['fixture_sha256']} code_point=U+200B offset=4")
    print("unicode_default_ignorable=present anthropic_provider=unknown")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, OSError, TypeError, ValueError) as error:
        print(f"claude watermark claim audit: FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
