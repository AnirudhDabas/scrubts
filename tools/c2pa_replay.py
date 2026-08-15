#!/usr/bin/env python3
"""Generate or check pinned C2PA corpus and selected-adversarial replay data."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "evidence" / "c2pa-replay-manifest.json"
CORPUS_RESULT = ROOT / "evidence" / "c2pa-corpus-results.json"
ADVERSARIAL_RESULT = ROOT / "evidence" / "c2pa-adversarial-results.json"
STATUS_IDS = {
    "scrub_parse_status": "c2pa.manifest_store",
    "scrub_validation_status": "c2pa.manifest_validation",
    "scrub_binding_status": "c2pa.hard_binding",
    "scrub_trust_status": "c2pa.credential_trust",
}
BIDI_CONTROLS = {
    0x061C,
    0x200E,
    0x200F,
    *range(0x2028, 0x202F),
    *range(0x2066, 0x206A),
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def forbidden_human_control(value: str) -> bool:
    return any(
        scalar != "\n"
        and (
            ord(scalar) < 0x20
            or 0x7F <= ord(scalar) <= 0x9F
            or ord(scalar) in BIDI_CONTROLS
        )
        for scalar in value
    )


def scrub_binary() -> Path:
    completed = subprocess.run(
        ["cargo", "build", "--offline", "--quiet", "-p", "scrub", "--bin", "scrub"],
        cwd=ROOT,
    )
    if completed.returncode != 0:
        raise RuntimeError("could not build scrub for C2PA replay")
    suffix = ".exe" if sys.platform == "win32" else ""
    return ROOT / "target" / "debug" / f"scrub{suffix}"


def inspect(binary: Path, path: Path, human: bool = False) -> bytes:
    command = [str(binary), "inspect", str(path)]
    if not human:
        command.append("--json")
    completed = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if completed.returncode != 0 or completed.stderr:
        diagnostic = completed.stderr.decode("utf-8", "replace")[-4000:]
        raise RuntimeError(f"scrub replay failed for {path}: {diagnostic}")
    return completed.stdout


def statuses(report: dict[str, Any]) -> dict[str, str]:
    findings = {item["mechanism"]["id"]: item["status"] for item in report["findings"]}
    return {field: findings[mechanism] for field, mechanism in STATUS_IDS.items()}


def corpus_document(manifest: dict[str, Any], binary: Path) -> dict[str, Any]:
    source = manifest["public_corpus"]
    fixtures = []
    for item in source["fixtures"]:
        path = ROOT / item["local_path"]
        input_sha256 = sha256_bytes(path.read_bytes())
        if input_sha256 != item["upstream_sha256"]:
            raise ValueError(f"fixture identity mismatch: {item['fixture_id']}")
        actual = statuses(json.loads(inspect(binary, path)))
        if actual != item["expected_contract"]:
            raise ValueError(f"corpus contract mismatch for {item['fixture_id']}: {actual}")
        fixtures.append(
            {
                "fixture_id": item["fixture_id"],
                "input_sha256": input_sha256,
                "license": source["license"],
                "limitation": item["limitation"],
                "scrub_binding_status": actual["scrub_binding_status"],
                "scrub_parse_status": actual["scrub_parse_status"],
                "scrub_trust_status": actual["scrub_trust_status"],
                "scrub_validation_status": actual["scrub_validation_status"],
                "unsupported_state": item["unsupported_state"],
                "upstream_category": item["upstream_category"],
                "upstream_commit": source["upstream_commit"],
                "upstream_path": item["upstream_path"],
                "upstream_repo": source["upstream_repo"],
                "upstream_sha256": item["upstream_sha256"],
                "upstream_spec_corpus_version": source["upstream_spec_corpus_version"],
                "contract_equal": True,
            }
        )
    return {
        "schema_version": "0.1",
        "evidence_scope": "external_corpus_integration",
        "source": {
            "integration": source["integration"],
            "license": source["license"],
            "upstream_commit": source["upstream_commit"],
            "upstream_repo": source["upstream_repo"],
            "upstream_spec_corpus_version": source["upstream_spec_corpus_version"],
        },
        "scrub": {
            "version": "0.1.0",
            "c2pa_dependency": "c2pa-rs 0.90.12",
            "replay_command": ["python", "tools/c2pa_replay.py", "--check"],
        },
        "fixtures": fixtures,
    }


def adversarial_document(manifest: dict[str, Any], binary: Path) -> dict[str, Any]:
    source = manifest["selected_adversarial"]
    fixtures = []
    generated = ROOT / "target" / "mega-b" / "c2pa-adversarial"
    generated.mkdir(parents=True, exist_ok=True)
    for item in source["cases"]:
        base = ROOT / item["base_fixture"]
        base_bytes = base.read_bytes()
        if sha256_bytes(base_bytes) != item["base_fixture_sha256"]:
            raise ValueError(f"base fixture identity mismatch: {item['case_id']}")
        search = bytes.fromhex(item["search_hex"])
        replacement = bytes.fromhex(item["replacement_hex"])
        if len(search) != len(replacement) or base_bytes.count(search) != 1:
            raise ValueError(f"ambiguous adversarial generation: {item['case_id']}")
        derivative = base_bytes.replace(search, replacement, 1)
        path = generated / f"{item['case_id']}.png"
        path.write_bytes(derivative)
        actual = statuses(json.loads(inspect(binary, path)))
        human = inspect(binary, path, human=True).decode("utf-8")
        actual.update(
            {
                "human_output_control_safe": not forbidden_human_control(human),
                "hostile_metadata_projected": replacement.decode("utf-8") in human,
            }
        )
        if actual != item["expected_contract"]:
            raise ValueError(f"adversarial contract mismatch for {item['case_id']}: {actual}")
        fixtures.append(
            {
                "fixture_id": item["case_id"],
                "attack_class": item["attack_class"],
                "attack_source": source["attack_source"],
                "attack_source_path": item["attack_source_path"],
                "attack_source_sha256": item["attack_source_sha256"],
                "generation_identity": item["generation_identity"],
                "input_sha256": sha256_bytes(derivative),
                "limitation": item["limitation"],
                "scrub_binding_status": actual["scrub_binding_status"],
                "scrub_parse_status": actual["scrub_parse_status"],
                "scrub_trust_status": actual["scrub_trust_status"],
                "scrub_validation_status": actual["scrub_validation_status"],
                "upstream_revision": source["upstream_revision"],
                "human_output_control_safe": actual["human_output_control_safe"],
                "hostile_metadata_projected": actual["hostile_metadata_projected"],
                "contract_equal": True,
            }
        )
    return {
        "schema_version": "0.1",
        "evidence_scope": "selected_adversarial_contracts",
        "source": {
            "attack_source": source["attack_source"],
            "license": source["license"],
            "upstream_revision": source["upstream_revision"],
        },
        "scrub": {
            "version": "0.1.0",
            "c2pa_dependency": "c2pa-rs 0.90.12",
            "replay_command": ["python", "tools/c2pa_replay.py", "--check"],
        },
        "fixtures": fixtures,
    }


def encoded(document: dict[str, Any]) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="write generated replay records")
    parser.add_argument("--check", action="store_true", help="check committed replay records")
    arguments = parser.parse_args()
    if arguments.write == arguments.check:
        parser.error("select exactly one of --write or --check")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    binary = scrub_binary()
    documents = [
        (CORPUS_RESULT, corpus_document(manifest, binary)),
        (ADVERSARIAL_RESULT, adversarial_document(manifest, binary)),
    ]
    for path, document in documents:
        value = encoded(document)
        if arguments.write:
            path.write_text(value, encoding="utf-8", newline="\n")
        elif not path.is_file() or path.read_text(encoding="utf-8") != value:
            raise ValueError(f"committed replay result is stale: {path.relative_to(ROOT)}")
    print("C2PA_REPLAY_OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
