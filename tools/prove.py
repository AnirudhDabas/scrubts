#!/usr/bin/env python3
"""Offline, deterministic orchestration for the repository claim ledger."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROOF_PATH = ROOT / "target" / "proof" / "proof.json"
# Keep the authoritative current-run marker writable even when the artifact
# output directory is unavailable.  Consumers must consult this marker before
# treating proof.json as the result of the current invocation.
PROOF_STATE_PATH = ROOT / "target" / "proof-control" / "proof-state.json"

PROOF_SOURCE_FILES = frozenset(
    {
        ".gitattributes",
        ".gitignore",
        "CONFORMANCE.md",
        "Cargo.lock",
        "Cargo.toml",
        "Justfile",
        "LICENSE",
        "THIRD_PARTY_NOTICES.md",
        "crates/scrub-report/Cargo.toml",
        "crates/scrub/Cargo.toml",
        "docs/adr/0004-release-provenance.md",
        "docs/RELEASE_INTEGRITY.md",
        "docs/specs/mega-b-adversarial-determinism.md",
        "docs/specs/mega-c-release-integrity.md",
        "docs/specs/product-proof.md",
        "docs/specs/report-schema.md",
        "research/sources.yaml",
    }
)
PROOF_SOURCE_PREFIXES = (
    ".github/workflows/",
    "crates/scrub-report/src/",
    "crates/scrub/examples/",
    "crates/scrub/src/",
    "crates/scrub/tests/",
    "evidence/",
    "fuzz/",
    "schemas/",
    "tools/",
)


def in_proof_source_scope(relative: str) -> bool:
    relative = relative.replace("\\", "/")
    return relative in PROOF_SOURCE_FILES or relative.startswith(PROOF_SOURCE_PREFIXES)


def waterlarp_python() -> Path:
    candidates = (
        ROOT / "waterlarp" / ".venv" / "Scripts" / "python.exe",
        ROOT / "waterlarp" / ".venv" / "bin" / "python",
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise RuntimeError(
        "WaterLARP virtual environment is missing; create waterlarp/.venv from "
        "waterlarp/requirements-lock.txt before running just prove"
    )


def ensure_validation_runtime() -> None:
    try:
        import jsonschema  # noqa: F401
        import yaml  # noqa: F401
    except ImportError:
        interpreter = waterlarp_python()
        if Path(sys.executable).resolve() == interpreter.resolve():
            raise
        completed = subprocess.run([str(interpreter), str(Path(__file__).resolve()), *sys.argv[1:]])
        raise SystemExit(completed.returncode)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path.relative_to(ROOT)} must contain a JSON object")
    return value


def write_run_state(result: str, reason: str) -> None:
    """Record whether the canonical proof belongs to the current invocation."""
    PROOF_STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(
        {
            "schema_version": "0.1",
            "result": result,
            "artifact": "proof.json",
            "reason": reason,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    temporary = PROOF_STATE_PATH.with_suffix(".tmp")
    temporary.write_text(encoded + "\n", encoding="utf-8", newline="\n")
    temporary.replace(PROOF_STATE_PATH)


def git_output(arguments: list[str]) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    ).stdout


def repository_identity() -> dict[str, Any]:
    base_revision = git_output(["rev-parse", "HEAD"]).strip()
    status_lines = git_output(["status", "--porcelain=v1", "--untracked-files=all"]).splitlines()
    relevant: list[dict[str, Any]] = []
    for line in status_lines:
        if len(line) < 4:
            continue
        status = line[:2]
        relative = line[3:]
        if " -> " in relative:
            relative = relative.split(" -> ", 1)[1]
        if not in_proof_source_scope(relative):
            continue
        path = ROOT / relative
        worktree_sha256 = sha256_file(path) if path.is_file() else None
        index_lines = git_output(["ls-files", "-s", "--", relative]).splitlines()
        index_object = index_lines[0].split()[1] if index_lines else None
        relevant.append(
            {
                "path": relative,
                "status": status,
                "index_object": index_object,
                "worktree_sha256": worktree_sha256,
            }
        )
    relevant.sort(key=lambda item: item["path"])
    state = "dirty" if relevant else "clean"
    staged = any(item["status"][0] not in {" ", "?"} for item in relevant)
    identity_document = {
        "scope": "proof_relevant_project",
        "base_revision": base_revision,
        "state": state,
        "staged": staged,
        "paths": relevant,
    }
    identity_bytes = json.dumps(
        identity_document, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return {
        "state": state,
        "base_revision": base_revision,
        "staged": staged,
        "scope": "proof_relevant_project",
        "identity_sha256": hashlib.sha256(identity_bytes).hexdigest(),
        "paths": relevant,
    }


def validate_with_schema(document: dict[str, Any], schema_path: Path) -> None:
    from jsonschema import Draft202012Validator

    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(document)


def source_records() -> dict[str, dict[str, Any]]:
    import yaml

    document = yaml.safe_load((ROOT / "research" / "sources.yaml").read_text(encoding="utf-8"))
    records = document.get("sources", [])
    result: dict[str, dict[str, Any]] = {}
    for record in records:
        source_id = record.get("id")
        if not isinstance(source_id, str) or source_id in result:
            raise ValueError("research/sources.yaml contains a missing or duplicate source ID")
        result[source_id] = record
    return result


def validate_ledger(ledger: dict[str, Any]) -> dict[str, dict[str, Any]]:
    validate_with_schema(ledger, ROOT / "schemas" / "claims-0.1.schema.json")
    for relative in (
        "evidence/c2pa-corpus-results.json",
        "evidence/c2pa-adversarial-results.json",
    ):
        validate_with_schema(
            load_json(ROOT / relative), ROOT / "schemas" / "c2pa-replay-0.1.schema.json"
        )
    sources = source_records()
    claims: dict[str, dict[str, Any]] = {}
    for claim in ledger["claims"]:
        claim_id = claim["claim_id"]
        if claim_id in claims:
            raise ValueError(f"duplicate claim ID: {claim_id}")
        claims[claim_id] = claim
        for relative in claim["evidence_artifacts"]:
            path = ROOT / relative
            if path.is_absolute() and not path.is_relative_to(ROOT):
                raise ValueError(f"claim {claim_id} has an external evidence path: {relative}")
            if not path.is_file():
                raise ValueError(f"claim {claim_id} evidence does not exist: {relative}")
        missing_sources = sorted(set(claim["authority_source_ids"]) - set(sources))
        if missing_sources:
            raise ValueError(f"claim {claim_id} has unknown authority sources: {missing_sources}")
        command = claim["reproduce_command"]
        if command[0].startswith("internal:"):
            if command not in [["internal:claim-ledger"], ["internal:report-contract"]]:
                raise ValueError(f"claim {claim_id} has an unknown internal oracle")
        elif command[0] in {"{waterlarp_python}", "{python}"}:
            if len(command) < 3:
                raise ValueError(f"claim {claim_id} has an incomplete Python command")
        elif command[0] != "cargo":
            raise ValueError(f"claim {claim_id} has an unsupported executable")
    return claims


def portable_environment() -> dict[str, str]:
    environment = os.environ.copy()
    source_root = str(ROOT / "waterlarp" / "src")
    existing = environment.get("PYTHONPATH")
    environment["PYTHONPATH"] = source_root if not existing else source_root + os.pathsep + existing
    environment["NO_COLOR"] = "1"
    return environment


def execute(command: list[str]) -> tuple[int, str]:
    resolved = list(command)
    if resolved[0] == "{waterlarp_python}":
        resolved[0] = str(waterlarp_python())
    elif resolved[0] == "{python}":
        resolved[0] = sys.executable
    try:
        completed = subprocess.run(
            resolved,
            cwd=ROOT,
            env=portable_environment(),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except OSError as error:
        return 127, str(error)
    if completed.returncode != 0:
        diagnostic = (completed.stdout + completed.stderr)[-4000:]
        return completed.returncode, diagnostic
    return 0, "command completed with the expected zero exit status"


def report_contract() -> tuple[int, str, dict[str, str] | None]:
    artifact = ROOT / "target" / "mega-a" / "report-contract.txt"
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact_bytes = b"ordinary deterministic text\n"
    artifact.write_bytes(artifact_bytes)
    command = [
        "cargo",
        "run",
        "--offline",
        "--quiet",
        "-p",
        "scrub",
        "--",
        "inspect",
        str(artifact.relative_to(ROOT)),
        "--json",
        "--explain",
    ]
    outputs: list[bytes] = []
    for _ in range(2):
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=portable_environment(),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if completed.returncode != 0:
            return completed.returncode, completed.stderr.decode("utf-8", "replace")[-4000:], None
        if completed.stderr:
            return 1, "scrub emitted diagnostics during a successful JSON inspection", None
        outputs.append(completed.stdout)
    if outputs[0] != outputs[1]:
        return 1, "repeated local JSON invocations were not byte-identical", None
    if b"\x1b" in outputs[0] or b"]8;" in outputs[0]:
        return 1, "JSON output contained ANSI or OSC control bytes", None
    try:
        report = json.loads(outputs[0])
        validate_with_schema(report, ROOT / "schemas" / "report-0.2.schema.json")
    except Exception as error:  # reported as an oracle failure, not a traceback
        return 1, f"report schema validation failed: {error}", None
    if report["artifact"]["path"] != "report-contract.txt":
        return 1, "JSON artifact display context leaked more than the stable file name", None
    prohibited_keys = {"timestamp", "hostname", "temporary_directory", "cwd"}

    def keys(value: Any) -> set[str]:
        if isinstance(value, dict):
            return set(value) | {key for member in value.values() for key in keys(member)}
        if isinstance(value, list):
            return {key for member in value for key in keys(member)}
        return set()

    leaked = sorted(keys(report) & prohibited_keys)
    if leaked:
        return 1, f"JSON report contains incidental machine keys: {leaked}", None
    digest = {
        "artifact_sha256": hashlib.sha256(artifact_bytes).hexdigest(),
        "report_sha256": hashlib.sha256(outputs[0]).hexdigest(),
        "scope": "local_repeatability_only",
    }
    return 0, "schema validation and two byte-identical local invocations passed", digest


def make_gate_result(claim: dict[str, Any], exit_code: int, reason: str) -> dict[str, Any]:
    return {
        "gate_id": claim["claim_id"],
        "claim_ids": [claim["claim_id"]],
        "status": "PASS" if exit_code == 0 else "FAIL",
        "command": claim["reproduce_command"],
        "expected_result": claim["expected_result"],
        "evidence_artifacts": claim["evidence_artifacts"],
        "limitations": claim["limitations"],
        "reason": reason if reason else f"oracle exited {exit_code}",
    }


def evaluate_claim(claim: dict[str, Any]) -> tuple[dict[str, Any], dict[str, str] | None]:
    command = claim["reproduce_command"]
    report_digest = None
    if command == ["internal:claim-ledger"]:
        exit_code, reason = 0, "schema, evidence paths, source IDs, and command contracts resolved"
    elif command == ["internal:report-contract"]:
        exit_code, reason, report_digest = report_contract()
    else:
        exit_code, reason = execute(command)
    return make_gate_result(claim, exit_code, reason), report_digest


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def validate_proof_semantics(proof: dict[str, Any], ledger: dict[str, Any]) -> None:
    """Validate semantic relationships not expressible by the JSON schema."""
    validate_with_schema(proof, ROOT / "schemas" / "proof-0.1.schema.json")
    expected_ids = [claim["claim_id"] for claim in ledger["claims"]]
    claim_ids = [claim["claim_id"] for claim in proof["claims"]]
    gate_ids = [gate["gate_id"] for gate in proof["gate_results"]]
    if len(claim_ids) != len(set(claim_ids)) or claim_ids != expected_ids:
        raise ValueError("proof claim IDs must uniquely and exactly match the ledger")
    if len(gate_ids) != len(set(gate_ids)) or gate_ids != expected_ids:
        raise ValueError("proof gate IDs must uniquely and exactly match the ledger")
    for claim, gate in zip(proof["claims"], proof["gate_results"], strict=True):
        if gate["claim_ids"] != [claim["claim_id"]]:
            raise ValueError(f"gate {gate['gate_id']} does not map to its claim")
        if claim["gate_status"] != gate["status"]:
            raise ValueError(f"claim {claim['claim_id']} disagrees with its gate")
    complete = all(gate["status"] == "PASS" for gate in proof["gate_results"])
    expected_result = "PROOF_COMPLETE" if complete else "PROOF_FAILED"
    if proof["result"] != expected_result:
        raise ValueError("proof result does not match all gate statuses")
    tested = proof["tested_source"]
    if tested["state"] not in {"clean", "dirty"}:
        raise ValueError("invalid tested source state")
    for entry in tested["paths"]:
        relative = entry["path"].replace("\\", "/")
        if not in_proof_source_scope(relative) or relative.startswith("../"):
            raise ValueError(f"tested source path is outside the project proof scope: {relative}")


def assemble_proof(
    ledger: dict[str, Any],
    gates: list[dict[str, Any]],
    report_digests: list[dict[str, str]],
) -> dict[str, Any]:
    sources = source_records()
    source_ids = sorted(
        {source_id for claim in ledger["claims"] for source_id in claim["authority_source_ids"]}
    )
    fixture_paths = sorted(
        {
            relative
            for claim in ledger["claims"]
            for relative in claim["evidence_artifacts"]
            if "/fixtures/" in relative.replace("\\", "/")
        }
    )
    statuses = {gate["gate_id"]: gate["status"] for gate in gates}
    tested_source = repository_identity()
    revision = tested_source["base_revision"]
    complete = all(gate["status"] == "PASS" for gate in gates)
    return {
        "schema_version": "0.1",
        "project_revision": revision,
        "tested_source": tested_source,
        "result": "PROOF_COMPLETE" if complete else "PROOF_FAILED",
        "claims": [
            {
                "claim_id": claim["claim_id"],
                "claim_status": claim["status"],
                "gate_status": statuses[claim["claim_id"]],
            }
            for claim in ledger["claims"]
        ],
        "source_identities": [
            {"source_id": source_id, "revision": str(sources[source_id].get("revision", ""))}
            for source_id in source_ids
        ],
        "fixture_identities": [
            {"path": relative, "sha256": sha256_file(ROOT / relative)}
            for relative in fixture_paths
        ],
        "gate_results": gates,
        "report_digests": sorted(report_digests, key=lambda item: item["artifact_sha256"]),
        "limitations": [
            "Default proof is offline and does not execute a pinned KGW upstream checkout.",
            "Ignored local WaterLARP pilot outputs are not proof prerequisites.",
            "Local report repeatability does not establish cross-platform determinism or RFC 8785 compliance.",
            "The default proof does not rerun or generalize the historical scoped Windows, Linux, and macOS comparison.",
            "Bounded fuzz smoke is a separate CI lane and does not prove absence of bugs.",
            "A passing UNKNOWN-boundary gate does not turn UNKNOWN into a negative provider result.",
        ],
    }


def write_proof(proof: dict[str, Any], ledger: dict[str, Any]) -> None:
    validate_proof_semantics(proof, ledger)
    PROOF_PATH.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(proof, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    temporary = PROOF_PATH.with_suffix(".tmp")
    temporary.write_text(encoded + "\n", encoding="utf-8", newline="\n")
    temporary.replace(PROOF_PATH)


def main() -> int:
    try:
        write_run_state("PROOF_RUNNING", "proof invocation started")
        ensure_validation_runtime()
        ledger = load_json(ROOT / "evidence" / "claims.json")
        validate_ledger(ledger)
    except Exception as error:
        try:
            write_run_state("PROOF_FAILED", f"proof setup failed: {error}")
        except Exception as state_error:
            print(f"proof state update failed: {state_error}", file=sys.stderr)
        print(f"proof setup failed: {error}", file=sys.stderr)
        return 1

    gates: list[dict[str, Any]] = []
    report_digests: list[dict[str, str]] = []
    print("scrub.ts proof")
    print("-" * 48)
    try:
        for claim in ledger["claims"]:
            gate, report_digest = evaluate_claim(claim)
            gates.append(gate)
            if report_digest is not None:
                report_digests.append(report_digest)
            print(f"{claim['claim_id']:<38} {gate['status']}")
            if gate["status"] == "FAIL":
                print(f"  {gate['reason']}", file=sys.stderr)

        proof = assemble_proof(ledger, gates, report_digests)
        write_proof(proof, ledger)
        write_run_state(proof["result"], f"current proof artifact result: {proof['result']}")
    except Exception as error:
        try:
            write_run_state("PROOF_FAILED", f"proof execution failed: {error}")
        except Exception as state_error:
            print(f"proof state update failed: {state_error}", file=sys.stderr)
        print(f"proof artifact validation failed: {error}", file=sys.stderr)
        return 1
    print()
    print("result:")
    print(f"    {proof['result']}")
    print(f"artifact: {PROOF_PATH.relative_to(ROOT).as_posix()}")
    return 0 if proof["result"] == "PROOF_COMPLETE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
