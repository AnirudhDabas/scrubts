from __future__ import annotations

import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import shutil
import sys
import tarfile
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "scrub_third_party_licenses", ROOT / "tools" / "third_party_licenses.py"
)
assert SPEC is not None and SPEC.loader is not None
licenses = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = licenses
SPEC.loader.exec_module(licenses)

RELEASE_SPEC = importlib.util.spec_from_file_location(
    "scrub_release_for_license_tests", ROOT / "tools" / "release.py"
)
assert RELEASE_SPEC is not None and RELEASE_SPEC.loader is not None
release = importlib.util.module_from_spec(RELEASE_SPEC)
sys.modules[RELEASE_SPEC.name] = release
RELEASE_SPEC.loader.exec_module(release)


class ThirdPartyLicenseBundleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.value = (ROOT / licenses.ARTIFACT_PATH).read_bytes()
        cls.packages, cls.texts = licenses.parse_bundle(cls.value)

    def render_bundle(self, packages: list[dict[str, object]]) -> bytes:
        referenced = {
            source_file["sha256"]
            for bundled in packages
            for source_file in bundled["files"]
        }
        output = bytearray(
            (
                "scrub.ts third-party license texts\n"
                f"format-version: {licenses.FORMAT_VERSION}\n"
                f"package-count: {licenses.EXPECTED_PACKAGE_COUNT}\n"
                f"file-mapping-count: {sum(len(item['files']) for item in packages)}\n"
                "source-contract: locked four-target normal/build production dependency union\n"
                "content-contract: exact UTF-8 bytes from canonical crate files or pinned upstream fallback files\n"
            ).encode("utf-8")
        )
        for bundled in packages:
            output.extend(b"@@PACKAGE " + licenses.json_line(bundled["package"]) + b"\n")
            for source_file in bundled["files"]:
                output.extend(b"@@FILE " + licenses.json_line(source_file) + b"\n")
        output.extend(b"@@END-MAP\n")
        for digest in sorted(referenced):
            value = self.texts[digest]
            output.extend(
                b"@@TEXT "
                + licenses.json_line({"byte_length": len(value), "sha256": digest})
                + b"\n"
                + value
                + b"\n@@END-TEXT\n"
            )
        return bytes(output)

    def package_record(
        self, packages: list[dict[str, object]], name: str, version: str
    ) -> dict[str, object]:
        return next(
            item
            for item in packages
            if (item["package"]["name"], item["package"]["version"])
            == (name, version)
        )

    def temporary_review_root(
        self, bundle: bytes, manifest: bytes | None = None
    ) -> tempfile.TemporaryDirectory[str]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for path in ("Cargo.lock", "THIRD_PARTY_NOTICES.md"):
            shutil.copyfile(ROOT / path, root / path)
        (root / licenses.ARTIFACT_PATH).write_bytes(bundle)
        manifest_path = root / licenses.MANIFEST_PATH
        manifest_path.parent.mkdir(parents=True)
        manifest_path.write_bytes(
            (ROOT / licenses.MANIFEST_PATH).read_bytes() if manifest is None else manifest
        )
        shutil.copytree(
            ROOT / "third_party" / "license-fallbacks",
            root / "third_party" / "license-fallbacks",
        )
        return temporary

    def assert_strict_and_release_reject(
        self, bundle: bytes, message: str
    ) -> None:
        with self.temporary_review_root(bundle) as directory_name:
            root = Path(directory_name)
            with self.assertRaisesRegex(licenses.LicenseBundleError, message):
                licenses.verify_reviewed_bundle(root)
            binary = root / "scrub"
            binary.write_bytes(b"synthetic binary\n")
            output = root / "release-output"
            stderr = io.StringIO()
            with mock.patch.object(release, "ROOT", root), mock.patch(
                "sys.stderr", stderr
            ):
                result = release.main(
                    [
                        "package",
                        "--binary",
                        str(binary),
                        "--target",
                        "x86_64-unknown-linux-gnu",
                        "--source-commit",
                        "a" * 40,
                        "--source-tree-state",
                        "dirty_worktree",
                        "--rustc-version",
                        "rustc 1.97.1 (8bab26f4f 2026-07-14)",
                        "--cargo-version",
                        "cargo 1.97.1 (c980f4866 2026-06-30)",
                        "--output-dir",
                        str(output),
                        "--preflight",
                    ]
                )
            self.assertEqual(result, 2)
            self.assertIn("third-party license bundle is invalid", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_exact_sources_regenerate_reviewed_bundle(self) -> None:
        self.assertEqual(licenses.verify_sources(), (251, 470, 185))

    def test_reviewed_bundle_is_complete_without_source_cache(self) -> None:
        self.assertEqual(licenses.verify_reviewed_bundle(), (251, 470, 185))

    def test_every_package_is_mapped_in_deterministic_order(self) -> None:
        packages, texts = licenses.parse_bundle(self.value)
        identities = [
            (item["package"]["name"], item["package"]["version"]) for item in packages
        ]
        self.assertEqual(identities, sorted(identities))
        self.assertEqual(len(identities), 251)
        self.assertEqual(len(texts), 185)

    def test_coset_authors_is_exact_and_reviewed(self) -> None:
        package = self.package_record(self.packages, "coset", "0.4.2")
        authors = next(item for item in package["files"] if item["filename"] == "AUTHORS")
        self.assertEqual(authors["byte_length"], 288)
        self.assertEqual(
            authors["sha256"],
            "ed66b6e0ab989ef7fe44696840bbc6509e850099df76bfff6fcc6954d01bb7ad",
        )
        self.assertEqual(hashlib.sha256(self.texts[authors["sha256"]]).hexdigest(), authors["sha256"])

    def test_independent_broad_archive_scan_has_no_omitted_conventional_files(self) -> None:
        broad = re.compile(
            r"^(?:licen[cs]e(?:[-._].*)?|copying(?:[-._].*)?|"
            r"copyrights?(?:[-._].*)?|notice(?:[-._].*)?|unlicense|authors)$",
            re.IGNORECASE,
        )
        inventory = licenses.parse_inventory()
        discovered: set[tuple[str, str, str, str]] = set()
        for key, package in inventory.items():
            root_name = f"{package.name}-{package.version}"
            with tarfile.open(licenses.crate_archive(package), "r:gz") as archive:
                for member in archive:
                    path = licenses.PurePosixPath(member.name)
                    if (
                        not member.isfile()
                        or len(path.parts) < 2
                        or path.parts[0] != root_name
                        or not broad.fullmatch(path.name)
                    ):
                        continue
                    extracted = archive.extractfile(member)
                    self.assertIsNotNone(extracted)
                    value = extracted.read()
                    value.decode("utf-8")
                    discovered.add((*key, licenses.PurePosixPath(*path.parts[1:]).as_posix(), hashlib.sha256(value).hexdigest()))
        reviewed = {
            (
                bundled["package"]["name"],
                bundled["package"]["version"],
                source_file["filename"],
                source_file["sha256"],
            )
            for bundled in self.packages
            for source_file in bundled["files"]
            if source_file["origin"].startswith("crate-package:")
        }
        self.assertEqual(discovered - reviewed, set())

    def test_ordinary_mapping_removal_fails_strict_and_release_paths(self) -> None:
        packages = copy.deepcopy(self.packages)
        abnf = self.package_record(packages, "abnf", "0.13.0")
        abnf["files"] = [
            item for item in abnf["files"] if item["filename"] != "LICENSE-APACHE"
        ]
        abnf["package"]["file_count"] = len(abnf["files"])
        self.assert_strict_and_release_reject(
            self.render_bundle(packages), "mapping disagrees with manifest"
        )

    def test_valid_existing_text_remapping_fails_strict_and_release_paths(self) -> None:
        packages = copy.deepcopy(self.packages)
        abnf = self.package_record(packages, "abnf", "0.13.0")
        source_file = abnf["files"][0]
        replacement = next(digest for digest in sorted(self.texts) if digest != source_file["sha256"])
        source_file["sha256"] = replacement
        source_file["text_id"] = f"sha256:{replacement}"
        source_file["byte_length"] = len(self.texts[replacement])
        self.assert_strict_and_release_reject(
            self.render_bundle(packages), "mapping disagrees with manifest"
        )

    def test_duplicate_mapping_fails_strict_and_release_paths(self) -> None:
        packages = copy.deepcopy(self.packages)
        abnf = self.package_record(packages, "abnf", "0.13.0")
        abnf["files"].insert(1, copy.deepcopy(abnf["files"][0]))
        abnf["package"]["file_count"] = len(abnf["files"])
        self.assert_strict_and_release_reject(
            self.render_bundle(packages), "duplicate license bundle package/file"
        )

    def test_missing_coset_authors_mapping_fails_closed(self) -> None:
        packages = copy.deepcopy(self.packages)
        coset = self.package_record(packages, "coset", "0.4.2")
        coset["files"] = [item for item in coset["files"] if item["filename"] != "AUTHORS"]
        coset["package"]["file_count"] = len(coset["files"])
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(self.render_bundle(packages))
            path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "mapping disagrees with manifest"):
                licenses.verify_reviewed_bundle(artifact_path=path)
        finally:
            path.unlink()

    def test_extra_mapping_fails_closed(self) -> None:
        packages = copy.deepcopy(self.packages)
        abnf = self.package_record(packages, "abnf", "0.13.0")
        extra = copy.deepcopy(abnf["files"][0])
        extra["filename"] = "NOTICE"
        extra["origin"] = "crate-package:NOTICE"
        abnf["files"].append(extra)
        abnf["files"].sort(key=lambda item: (item["filename"].casefold(), item["filename"]))
        abnf["package"]["file_count"] = len(abnf["files"])
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(self.render_bundle(packages))
            path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "mapping disagrees with manifest"):
                licenses.verify_reviewed_bundle(artifact_path=path)
        finally:
            path.unlink()

    def test_manifest_bundle_digest_mismatch_fails_closed(self) -> None:
        document = json.loads((ROOT / licenses.MANIFEST_PATH).read_text(encoding="utf-8"))
        coset = next(
            item for item in document["packages"] if (item["name"], item["version"]) == ("coset", "0.4.2")
        )
        authors = next(item for item in coset["files"] if item["filename"] == "AUTHORS")
        authors["sha256"] = "f" * 64
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(licenses.json_bytes(document))
            path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "mapping disagrees with manifest"):
                licenses.verify_reviewed_bundle(manifest_path=path)
        finally:
            path.unlink()

    def test_exact_upstream_zero_file_results_are_explicit(self) -> None:
        packages, _texts = licenses.parse_bundle(self.value)
        zero_file = {
            (item["package"]["name"], item["package"]["version"])
            for item in packages
            if item["package"]["file_count"] == 0
        }
        self.assertEqual(
            zero_file,
            {
                ("btree-range-map", "0.7.2"),
                ("range-traits", "0.3.2"),
                ("static-regular-grammar", "2.0.2"),
            },
        )

    def test_text_digest_mutation_fails_closed(self) -> None:
        marker = self.value.index(b"@@TEXT ")
        content = self.value.index(b"\n", marker) + 1
        changed = bytearray(self.value)
        changed[content] ^= 1
        with self.assertRaisesRegex(licenses.LicenseBundleError, "digest disagrees"):
            licenses.parse_bundle(bytes(changed))

    def test_file_digest_mutation_fails_closed(self) -> None:
        marker = self.value.index(b"@@TEXT ")
        digest_at = self.value.index(b'"sha256":"', marker) + len(b'"sha256":"')
        changed = (
            self.value[:digest_at]
            + b"0" * 64
            + self.value[digest_at + 64 :]
        )
        with self.assertRaisesRegex(licenses.LicenseBundleError, "digest disagrees"):
            licenses.parse_bundle(changed)

    def test_missing_package_record_fails_closed(self) -> None:
        first = self.value.index(b"@@PACKAGE ")
        second = self.value.index(b"@@PACKAGE ", first + 1)
        with self.assertRaisesRegex(licenses.LicenseBundleError, "mapping count|package records"):
            licenses.parse_bundle(self.value[:first] + self.value[second:])

    def test_unexpected_package_and_traversal_filename_fail_closed(self) -> None:
        packages = copy.deepcopy(self.packages)
        packages[0]["package"]["name"] = "unexpected-package"
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(self.render_bundle(packages))
            unexpected_path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "extra package"):
                licenses.verify_reviewed_bundle(artifact_path=unexpected_path)
        finally:
            unexpected_path.unlink()
        packages = copy.deepcopy(self.packages)
        source_file = packages[0]["files"][0]
        source_file["filename"] = "../LICENSE"
        source_file["origin"] = "crate-package:../LICENSE"
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(self.render_bundle(packages))
            traversal_path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "unsafe"):
                licenses.verify_reviewed_bundle(artifact_path=traversal_path)
        finally:
            traversal_path.unlink()

    def test_package_reordering_fails_closed(self) -> None:
        packages = copy.deepcopy(self.packages)
        packages[0], packages[1] = packages[1], packages[0]
        with tempfile.NamedTemporaryFile(delete=False) as output:
            output.write(self.render_bundle(packages))
            path = Path(output.name)
        try:
            with self.assertRaisesRegex(licenses.LicenseBundleError, "membership or ordering"):
                licenses.verify_reviewed_bundle(artifact_path=path)
        finally:
            path.unlink()


if __name__ == "__main__":
    unittest.main()
