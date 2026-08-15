"""Machine-clean WaterLARP command line interface."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import asdict
from pathlib import Path

from waterlarp.authority import AUTHORITY_RECORDS
from waterlarp.authority_sources import load_source_ledger, verify_artifacts
from waterlarp.doctor import doctor_report
from waterlarp.manifests import canonical_json_bytes


def _emit(value: object) -> None:
    sys.stdout.buffer.write(canonical_json_bytes(value, terminal_newline=True))


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _aggregate_run(run_dir: Path) -> dict[str, object]:
    from waterlarp.manifests import verify_artifact_checksums
    from waterlarp.metrics.aggregate import aggregate_records

    manifest_path = run_dir / "manifest.json"
    examples_path = run_dir / "examples.jsonl"
    if not manifest_path.is_file() or not examples_path.is_file():
        raise FileNotFoundError("run must contain manifest.json and examples.jsonl")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    records = [
        json.loads(line)
        for line in examples_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if any(record.get("run_id") != manifest["run_id"] for record in records):
        raise ValueError("example run_id does not match manifest")
    verify_artifact_checksums(run_dir)
    aggregate_path = run_dir / "aggregate.json"
    aggregates = aggregate_records(records, float(manifest["calibration_policy"]["target_fpr"]))
    serializable = json.loads(json.dumps(aggregates, default=asdict))
    if canonical_json_bytes(serializable, terminal_newline=True) != aggregate_path.read_bytes():
        raise ValueError("stored aggregate is not reproducible from canonical examples")
    return {
        "status": "AGGREGATE_VERIFIED",
        "run_id": manifest["run_id"],
        "record_count": len(records),
        "aggregate_count": len(aggregates),
        "aggregate_path": str(aggregate_path),
    }


def _verify_run(run_dir: Path, kgw_checkout: Path) -> dict[str, object]:
    from transformers import AutoTokenizer

    from waterlarp.adapters.kgw import KgwAdapter, KgwConfig
    from waterlarp.adapters.synthid import SynthIdAdapter, SynthIdConfig
    from waterlarp.evidence import rescore_canonical_record
    from waterlarp.manifests import verify_artifact_checksums

    verify_artifact_checksums(run_dir)
    manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
    records = [
        json.loads(line)
        for line in (run_dir / "examples.jsonl").read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    model_repo = str(manifest["model_repo"])
    cache_name = "models--" + model_repo.replace("/", "--")
    snapshot = (
        _repository_root()
        / "waterlarp"
        / ".cache"
        / "huggingface"
        / "hub"
        / cache_name
        / "snapshots"
        / manifest["model_revision"]
    )
    tokenizer = AutoTokenizer.from_pretrained(snapshot, local_files_only=True)
    kgw_metadata = manifest["detector_config"]["reference.kgw"]
    synthid_metadata = manifest["detector_config"]["reference.synthid_text"]
    kgw = KgwAdapter(
        KgwConfig(
            gamma=kgw_metadata["gamma"],
            delta=kgw_metadata["delta"],
            context_width=kgw_metadata["context_width"],
            prf_type=kgw_metadata["prf_type"],
            self_salt=kgw_metadata["self_salt"],
            base_key=kgw_metadata["base_key"],
            ignore_repeated_ngrams=kgw_metadata["ignore_repeated_ngrams"],
            device=kgw_metadata["device"],
            rng=kgw_metadata["rng"],
        ),
        kgw_checkout,
    )
    synthid = SynthIdAdapter(
        SynthIdConfig(
            keys=tuple(synthid_metadata["keys"]),
            ngram_len=synthid_metadata["ngram_len"],
            sampling_table_size=synthid_metadata["sampling_table_size"],
            sampling_table_seed=synthid_metadata["sampling_table_seed"],
            context_history_size=synthid_metadata["context_history_size"],
            device=synthid_metadata["device"],
            detector=synthid_metadata["detector"],
            length_calibration=synthid_metadata["length_calibration"],
            num_leaves=synthid_metadata["num_leaves"],
        )
    )
    adapters = {"reference.kgw": kgw, "reference.synthid_text": synthid}
    results = [
        rescore_canonical_record(
            record,
            adapters=adapters,
            tokenizer=tokenizer,
            threshold_records=manifest["threshold_records"],
        )
        for record in records
    ]
    return {
        "status": "RUN_VERIFIED",
        "run_id": manifest["run_id"],
        "artifact_count": len(manifest["scientific_artifact_sha256"]),
        "record_count": len(results),
        "p_value_count": sum(result.p_value is not None for result in results),
        "threshold_lookup_count": sum(result.threshold_id is not None for result in results),
        "decision_count": sum(result.decision is not None for result in results),
    }


def _run_parity(kind: str, checkout: Path | None) -> int:
    package_root = Path(__file__).resolve().parents[2]
    args = [
        sys.executable,
        "-m",
        "pytest",
        "-q",
        "tests/test_parity_synthid.py" if kind == "synthid" else "tests/test_parity_kgw.py",
    ]
    environment = os.environ.copy()
    if kind == "kgw":
        if checkout is None:
            raise ValueError(
                "KGW parity requires --checkout to a pinned author-repository checkout"
            )
        environment["WATERLARP_KGW_CHECKOUT"] = str(checkout)
    return subprocess.run(args, cwd=package_root, env=environment, check=False).returncode


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(prog="waterlarp")
    commands = root.add_subparsers(dest="command", required=True)
    commands.add_parser("doctor")
    source = commands.add_parser("sources")
    source_commands = source.add_subparsers(dest="source_command", required=True)
    verify = source_commands.add_parser("verify")
    verify.add_argument("--kgw-checkout", type=Path)
    verify.add_argument("--synthid-checkout", type=Path)
    verify.add_argument("--transformers-checkout", type=Path)
    parity = commands.add_parser("parity")
    parity.add_argument("kind", choices=("kgw", "synthid"))
    parity.add_argument("--checkout", type=Path)
    run = commands.add_parser("run")
    run.add_argument("--config", type=Path, required=True)
    run.add_argument("--kgw-checkout", type=Path)
    run.add_argument("--output-root", type=Path, default=Path("results/local"))
    aggregate = commands.add_parser("aggregate")
    aggregate.add_argument("--run", type=Path, required=True)
    verify_run = commands.add_parser("verify-run")
    verify_run.add_argument("--run", type=Path, required=True)
    verify_run.add_argument("--kgw-checkout", type=Path, required=True)
    validate_run = commands.add_parser("validate-run")
    validate_run.add_argument("--run", type=Path, required=True)
    return root


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "doctor":
            _emit(doctor_report())
            return 0
        if arguments.command == "sources":
            ledger = load_source_ledger(_repository_root() / "research" / "sources.yaml")
            checkouts = {
                key: value
                for key, value in {
                    "kgw": arguments.kgw_checkout,
                    "synthid-text": arguments.synthid_checkout,
                    "synthid-text-transformers": arguments.transformers_checkout,
                }.items()
                if value is not None
            }
            artifact_results = verify_artifacts(ledger, checkouts or None)
            for record in AUTHORITY_RECORDS.values():
                record.validate()
            _emit(
                {
                    "source_count": len(ledger["sources"]),
                    "unique_ids": True,
                    "artifacts": artifact_results,
                }
            )
            return 0
        if arguments.command == "parity":
            return _run_parity(arguments.kind, arguments.checkout)
        if arguments.command == "run":
            if arguments.kgw_checkout is None:
                raise ValueError("pilot run requires --kgw-checkout")
            from waterlarp.pilot import run_pilot

            run_dir = run_pilot(arguments.config, arguments.kgw_checkout, arguments.output_root)
            _emit({"status": "PILOT_COMPLETE", "run_directory": str(run_dir)})
            return 0
        if arguments.command == "aggregate":
            _emit(_aggregate_run(arguments.run))
            return 0
        if arguments.command == "verify-run":
            _emit(_verify_run(arguments.run, arguments.kgw_checkout))
            return 0
        if arguments.command == "validate-run":
            from waterlarp.schema_validation import validate_run_schemas

            counts = validate_run_schemas(arguments.run)
            _emit(
                {
                    "status": "SCHEMAS_VALID",
                    "object_count": sum(counts.values()),
                    **counts,
                }
            )
            return 0
    except (ValueError, FileNotFoundError, NotImplementedError) as exc:
        print(f"waterlarp: {exc}", file=sys.stderr)
        return 2
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
