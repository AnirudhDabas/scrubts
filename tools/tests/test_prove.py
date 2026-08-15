from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("scrub_prove", ROOT / "tools" / "prove.py")
assert SPEC is not None and SPEC.loader is not None
prove = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = prove
SPEC.loader.exec_module(prove)


def claim() -> dict[str, object]:
    return {
        "claim_id": "test.oracle",
        "status": "established",
        "reproduce_command": ["cargo", "test"],
        "expected_result": "oracle succeeds",
        "evidence_artifacts": ["evidence/claims.json"],
        "authority_source_ids": [],
        "limitations": [],
    }


class ProofResultTests(unittest.TestCase):
    def test_zero_exit_is_the_only_path_to_pass(self) -> None:
        self.assertEqual(prove.make_gate_result(claim(), 0, "ok")["status"], "PASS")
        self.assertEqual(prove.make_gate_result(claim(), 1, "failed")["status"], "FAIL")
        self.assertEqual(prove.make_gate_result(claim(), 127, "missing")["status"], "FAIL")

    def test_failed_required_gate_produces_failed_proof(self) -> None:
        ledger = {"claims": [claim()]}
        original_sources = prove.source_records
        original_sha256 = prove.sha256_file
        original_run = prove.subprocess.run
        prove.source_records = lambda: {}
        prove.sha256_file = lambda _path: "0" * 64

        class Revision:
            stdout = "80e31532179be8658d3f0a4c33c99b8bad885121\n"

        prove.subprocess.run = lambda *args, **kwargs: Revision()
        try:
            gate = prove.make_gate_result(claim(), 2, "oracle exited 2")
            proof = prove.assemble_proof(ledger, [gate], [])
        finally:
            prove.source_records = original_sources
            prove.sha256_file = original_sha256
            prove.subprocess.run = original_run
        self.assertEqual(proof["result"], "PROOF_FAILED")
        self.assertEqual(proof["claims"][0]["gate_status"], "FAIL")
        self.assertEqual(proof["gate_results"][0]["reason"], "oracle exited 2")

    def test_all_successful_required_gates_complete_proof(self) -> None:
        ledger = {"claims": [claim()]}
        original_sources = prove.source_records
        original_sha256 = prove.sha256_file
        original_run = prove.subprocess.run
        prove.source_records = lambda: {}
        prove.sha256_file = lambda _path: "0" * 64

        class Revision:
            stdout = "80e31532179be8658d3f0a4c33c99b8bad885121\n"

        prove.subprocess.run = lambda *args, **kwargs: Revision()
        try:
            gate = prove.make_gate_result(claim(), 0, "oracle passed")
            proof = prove.assemble_proof(ledger, [gate], [])
        finally:
            prove.source_records = original_sources
            prove.sha256_file = original_sha256
            prove.subprocess.run = original_run
        self.assertEqual(proof["result"], "PROOF_COMPLETE")
        self.assertEqual(proof["claims"][0]["gate_status"], "PASS")

    def test_semantic_validator_rejects_complete_proof_with_failed_gate(self) -> None:
        ledger = {"claims": [claim()]}
        gate = prove.make_gate_result(claim(), 1, "failed")
        proof = {
            "schema_version": "0.1",
            "project_revision": "0" * 40,
            "tested_source": {
                "state": "clean", "base_revision": "0" * 40,
                "staged": False, "scope": "mega_a_product_proof",
                "identity_sha256": "0" * 64, "paths": [],
            },
            "result": "PROOF_COMPLETE",
            "claims": [{"claim_id": "test.oracle", "claim_status": "established", "gate_status": "FAIL"}],
            "source_identities": [], "fixture_identities": [],
            "gate_results": [gate], "report_digests": [], "limitations": [],
        }
        original_schema = prove.validate_with_schema
        prove.validate_with_schema = lambda *_args: None
        try:
            with self.assertRaises(ValueError):
                prove.validate_proof_semantics(proof, ledger)
        finally:
            prove.validate_with_schema = original_schema

    def test_semantic_validator_rejects_duplicate_gate_ids(self) -> None:
        ledger = {"claims": [claim()]}
        gate = prove.make_gate_result(claim(), 0, "ok")
        proof = prove.assemble_proof(ledger, [gate], [])
        proof["gate_results"].append(dict(gate))
        original_schema = prove.validate_with_schema
        prove.validate_with_schema = lambda *_args: None
        try:
            with self.assertRaises(ValueError):
                prove.validate_proof_semantics(proof, ledger)
        finally:
            prove.validate_with_schema = original_schema

    def test_repository_identity_excludes_unrelated_status(self) -> None:
        original_git = prove.git_output
        original_sha = prove.sha256_file
        def fake_git(arguments: list[str]) -> str:
            if arguments == ["rev-parse", "HEAD"]:
                return "0" * 40 + "\n"
            if arguments[:2] == ["status", "--porcelain=v1"]:
                return " M crates/scrub/src/main.rs\n?? BENCHMARKS.md\n"
            if arguments[:2] == ["ls-files", "-s"]:
                return "100644 " + "1" * 40 + " 0\tcrates/scrub/src/main.rs\n"
            raise AssertionError(arguments)
        prove.git_output = fake_git
        prove.sha256_file = lambda _path: "2" * 64
        try:
            identity = prove.repository_identity()
        finally:
            prove.git_output = original_git
            prove.sha256_file = original_sha
        self.assertEqual(identity["state"], "dirty")
        self.assertEqual([item["path"] for item in identity["paths"]], ["crates/scrub/src/main.rs"])

    def test_omitted_production_source_changes_identity(self) -> None:
        original_git = prove.git_output
        original_sha = prove.sha256_file
        source = "crates/scrub/src/unicode_normalization.rs"
        changed = [False]

        def fake_git(arguments: list[str]) -> str:
            if arguments == ["rev-parse", "HEAD"]:
                return "0" * 40 + "\n"
            if arguments[:2] == ["status", "--porcelain=v1"]:
                return f" M {source}\n?? BENCHMARKS.md\n"
            if arguments[:2] == ["ls-files", "-s"]:
                return "100644 " + "1" * 40 + " 0\t" + source + "\n"
            raise AssertionError(arguments)

        prove.git_output = fake_git
        prove.sha256_file = lambda _path: ("2" if not changed[0] else "3") * 64
        try:
            before = prove.repository_identity()
            changed[0] = True
            after = prove.repository_identity()
        finally:
            prove.git_output = original_git
            prove.sha256_file = original_sha

        self.assertEqual(after["state"], "dirty")
        self.assertFalse(after["staged"])
        self.assertEqual([item["path"] for item in after["paths"]], [source])
        self.assertNotEqual(before["identity_sha256"], after["identity_sha256"])

    def test_setup_failure_invalidates_previous_success(self) -> None:
        with self.subTest("stale artifact"):
            import tempfile
            with tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                original_path, original_state = prove.PROOF_PATH, prove.PROOF_STATE_PATH
                original_runtime, original_load = prove.ensure_validation_runtime, prove.load_json
                prove.PROOF_PATH = directory / "proof.json"
                prove.PROOF_STATE_PATH = directory / "proof-state.json"
                prove.PROOF_PATH.write_text('{"result":"PROOF_COMPLETE"}', encoding="utf-8")
                prove.ensure_validation_runtime = lambda: None
                prove.load_json = lambda _path: (_ for _ in ()).throw(ValueError("bad ledger"))
                try:
                    self.assertNotEqual(prove.main(), 0)
                    state = json.loads(prove.PROOF_STATE_PATH.read_text(encoding="utf-8"))
                finally:
                    prove.PROOF_PATH, prove.PROOF_STATE_PATH = original_path, original_state
                    prove.ensure_validation_runtime, prove.load_json = original_runtime, original_load
                self.assertEqual(state["result"], "PROOF_FAILED")

    def test_real_output_destination_failure_invalidates_previous_success(self) -> None:
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            artifact_dir = directory / "artifact-output"
            control_dir = directory / "control"
            artifact_dir.mkdir()
            control_dir.mkdir()
            original_path, original_state = prove.PROOF_PATH, prove.PROOF_STATE_PATH
            original_runtime, original_load = prove.ensure_validation_runtime, prove.load_json
            original_validate = prove.validate_ledger
            original_schema = prove.validate_with_schema
            prove.PROOF_PATH = artifact_dir / "proof.json"
            prove.PROOF_STATE_PATH = control_dir / "proof-state.json"
            prove.PROOF_PATH.write_text('{"result":"PROOF_COMPLETE"}', encoding="utf-8")
            # A real filesystem collision makes the atomic temporary output
            # path unwritable while the separate control directory remains
            # writable on Windows, Linux, and macOS.
            prove.PROOF_PATH.with_suffix(".tmp").mkdir()
            prove.ensure_validation_runtime = lambda: None
            prove.load_json = lambda _path: {"claims": []}
            prove.validate_ledger = lambda ledger: {}
            prove.validate_with_schema = lambda *_args: None
            try:
                self.assertNotEqual(prove.main(), 0)
                state = json.loads(prove.PROOF_STATE_PATH.read_text(encoding="utf-8"))
                old_artifact = json.loads(prove.PROOF_PATH.read_text(encoding="utf-8"))
            finally:
                prove.PROOF_PATH, prove.PROOF_STATE_PATH = original_path, original_state
                prove.ensure_validation_runtime, prove.load_json = original_runtime, original_load
                prove.validate_ledger = original_validate
                prove.validate_with_schema = original_schema
            self.assertEqual(state["result"], "PROOF_FAILED")
            self.assertEqual(old_artifact["result"], "PROOF_COMPLETE")
            self.assertNotEqual(state["result"], old_artifact["result"])


if __name__ == "__main__":
    unittest.main()
