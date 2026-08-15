from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
PLATFORM_SCHEMA = ROOT / "schemas" / "determinism-platform-0.1.schema.json"
SPEC = importlib.util.spec_from_file_location(
    "compare_determinism", ROOT / "tools" / "compare_determinism.py"
)
assert SPEC is not None and SPEC.loader is not None
compare_determinism = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare_determinism)


def result(platform: str, input_digest: str = "1" * 64, report_digest: str = "2" * 64) -> dict:
    return {
        "schema_version": "0.1",
        "project_revision": "3" * 40,
        "platform": platform,
        "fixtures": [
            {
                "fixture_id": "fixture",
                "input_sha256": input_digest,
                "expected_capability": "test capability",
                "semantic_report_sha256": report_digest,
            }
        ],
    }


class DeterminismComparatorTests(unittest.TestCase):
    def test_platform_schema_has_only_canonical_native_identities(self) -> None:
        schema = json.loads(PLATFORM_SCHEMA.read_text(encoding="utf-8"))
        allowed = schema["properties"]["platform"]["enum"]
        self.assertEqual(allowed, ["windows", "linux", "macos"])
        for invalid in ["local-windows", "win32", "darwin", "arbitrary"]:
            self.assertNotIn(invalid, allowed)

    def write_results(self, root: Path, values: list[dict]) -> None:
        for value in values:
            path = root / value["platform"] / "determinism-platform.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(value), encoding="utf-8")

    def test_three_equal_platform_results_establish_equality(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_results(root, [result(platform) for platform in compare_determinism.PLATFORMS])
            matrix, equal = compare_determinism.compare(compare_determinism.load_results(root))
        self.assertTrue(equal)
        self.assertEqual(matrix["equality_status"], "ESTABLISHED")
        self.assertEqual(matrix["platforms"], ["linux", "macos", "windows"])

    def test_input_identity_mismatch_fails_before_report_comparison(self) -> None:
        values = [result("linux"), result("macos"), result("windows", input_digest="4" * 64)]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_results(root, values)
            loaded = compare_determinism.load_results(root)
            with self.assertRaisesRegex(ValueError, "input bytes differ before semantic comparison"):
                compare_determinism.compare(loaded)

    def test_missing_platform_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_results(root, [result("linux"), result("windows")])
            with self.assertRaisesRegex(ValueError, "expected all platforms"):
                compare_determinism.load_results(root)


if __name__ == "__main__":
    unittest.main()
