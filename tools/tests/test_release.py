from __future__ import annotations

import copy
import gzip
import importlib.util
import io
import json
from pathlib import Path
import re
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock
import zipfile


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("scrub_release", ROOT / "tools" / "release.py")
assert SPEC is not None and SPEC.loader is not None
release = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release
SPEC.loader.exec_module(release)

SOURCE_COMMIT = "a" * 40
RUSTC_VERSION = "rustc 1.97.1 (8bab26f4f 2026-07-14)"
CARGO_VERSION = "cargo 1.97.1 (c980f4866 2026-06-30)"


class ReleaseContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.binary = self.directory / "synthetic-scrub"
        self.binary.write_bytes(b"synthetic scrub binary\n")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    @staticmethod
    def committed_source_bytes(
        _source_commit: str, path: str, root: Path = ROOT
    ) -> bytes:
        return (root / path).read_bytes()

    def metadata(self, target: str = release.TARGETS[0]) -> dict[str, object]:
        with mock.patch.object(
            release,
            "committed_release_source_bytes",
            side_effect=self.committed_source_bytes,
        ):
            return release.build_metadata(
                binary=self.binary,
                target=target,
                source_commit=SOURCE_COMMIT,
                source_tree_state="clean_commit",
                rustc_version=RUSTC_VERSION,
                cargo_version=CARGO_VERSION,
                tag=None,
            )

    def package(self, target: str, output: Path | None = None) -> Path:
        if output is None:
            output = self.directory / "packages"
        binary = self.directory / ("scrub.exe" if target.endswith("windows-msvc") else "scrub")
        binary.write_bytes((target + "\n").encode("ascii"))
        with mock.patch.object(
            release,
            "committed_release_source_bytes",
            side_effect=self.committed_source_bytes,
        ):
            return release.package_archive(
                binary=binary,
                target=target,
                source_commit=SOURCE_COMMIT,
                source_tree_state="clean_commit",
                rustc_version=RUSTC_VERSION,
                cargo_version=CARGO_VERSION,
                tag=None,
                output_dir=output,
            )

    def write_zip_variant(
        self,
        metadata: dict[str, object],
        *,
        binary_type: int = stat.S_IFREG,
        create_system: int = 3,
        file_compression: int = zipfile.ZIP_DEFLATED,
        metadata_bytes: bytes | None = None,
        reverse_order: bool = False,
    ) -> Path:
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        if metadata_bytes is not None:
            members[f"{archive_root}/RELEASE-METADATA.json"] = (metadata_bytes, 0o644)
        archive = self.directory / release.expected_archive_name("0.1.0", release.TARGETS[0])
        with zipfile.ZipFile(archive, "w") as output:
            directory = zipfile.ZipInfo(f"{archive_root}/", (1980, 1, 1, 0, 0, 0))
            directory.create_system = create_system
            directory.compress_type = zipfile.ZIP_STORED
            directory.external_attr = (stat.S_IFDIR | 0o755) << 16 | 0x10
            output.writestr(directory, b"")
            for name in sorted(members, reverse=reverse_order):
                data, mode = members[name]
                info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                info.create_system = create_system
                info.compress_type = file_compression
                file_type = binary_type if name.endswith("/scrub.exe") else stat.S_IFREG
                info.external_attr = (file_type | mode) << 16
                output.writestr(info, data)
        return archive

    def complete_release(self) -> tuple[Path, Path]:
        packages = self.directory / "packages"
        for target in release.TARGETS:
            self.package(target, packages)
        assembled = self.directory / "assembled"
        release.assemble_release(
            input_dir=packages,
            output_dir=assembled,
            source_commit=SOURCE_COMMIT,
            source_tree_state="clean_commit",
            tag=None,
        )
        return packages, assembled

    def rewrite_manifest_and_checksums(
        self, packages: Path, assembled: Path, document: dict[str, object]
    ) -> None:
        manifest = assembled / "release-manifest.json"
        release.write_atomic(manifest, release.json_bytes(document))
        archives = [packages / row["archive_filename"] for row in document["artifacts"]]
        release.write_atomic(
            assembled / "SHA256SUMS", release.checksum_bytes([*archives, manifest])
        )

    def test_tag_version_mismatch_and_malformed_tag_are_rejected(self) -> None:
        with self.assertRaisesRegex(release.ReleaseError, "does not match"):
            release.validate_version_contract("v0.2.0")
        for malformed in ("0.1.0", "v01.0.0", "release-v0.1.0", "v0.1"):
            with self.subTest(tag=malformed), self.assertRaisesRegex(
                release.ReleaseError, "malformed"
            ):
                release.validate_version_contract(malformed)

    def test_exact_byte_release_sources_are_not_git_text_paths(self) -> None:
        completed = subprocess.run(
            ["git", "check-attr", "text", "--", *release.RELEASE_SOURCE_BYTE_PATHS],
            cwd=ROOT,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(
            completed.stdout.splitlines(),
            [f"{path}: text: unset" for path in release.RELEASE_SOURCE_BYTE_PATHS],
        )

    def test_clean_commit_requires_exact_release_source_blob_bytes(self) -> None:
        root = self.directory / "source"
        root.mkdir()
        committed = {
            "Cargo.lock": b"version = 4\n",
            "LICENSE": b"license line\n",
            "THIRD_PARTY_NOTICES.md": b"notice line\n",
        }
        for path, value in committed.items():
            (root / path).write_bytes(value)

        with mock.patch.object(
            release,
            "committed_release_source_bytes",
            side_effect=lambda _commit, path, _root: committed[path],
        ):
            self.assertEqual(
                release.load_release_source_bytes(SOURCE_COMMIT, "clean_commit", root),
                committed,
            )
            for path in release.RELEASE_SOURCE_BYTE_PATHS:
                with self.subTest(path=path):
                    (root / path).write_bytes(committed[path].replace(b"\n", b"\r\n"))
                    with self.assertRaisesRegex(
                        release.ReleaseError, rf"clean-commit {re.escape(path)} bytes disagree"
                    ):
                        release.load_release_source_bytes(
                            SOURCE_COMMIT, "clean_commit", root
                        )
                    (root / path).write_bytes(committed[path])

    def test_clean_commit_fails_closed_when_a_source_blob_cannot_be_read(self) -> None:
        root = self.directory / "source"
        root.mkdir()
        for path in release.RELEASE_SOURCE_BYTE_PATHS:
            (root / path).write_bytes(b"working bytes\n")
        with mock.patch.object(
            release.subprocess,
            "run",
            side_effect=subprocess.CalledProcessError(128, ["git", "cat-file"]),
        ), self.assertRaisesRegex(release.ReleaseError, "cannot read Cargo.lock"):
            release.load_release_source_bytes(SOURCE_COMMIT, "clean_commit", root)

    def test_dirty_worktree_accepts_release_source_bytes_that_differ_from_head(self) -> None:
        root = self.directory / "source"
        root.mkdir()
        expected = {path: b"dirty working bytes\r\n" for path in release.RELEASE_SOURCE_BYTE_PATHS}
        for path, value in expected.items():
            (root / path).write_bytes(value)
        with mock.patch.object(release, "committed_release_source_bytes") as committed:
            self.assertEqual(
                release.load_release_source_bytes(SOURCE_COMMIT, "dirty_worktree", root),
                expected,
            )
        committed.assert_not_called()

    def test_package_uses_validated_support_file_bytes_without_eol_conversion(self) -> None:
        root = self.directory / "source"
        for package in ("scrub", "scrub-report"):
            manifest = root / "crates" / package / "Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_bytes(b'[package]\nversion = "0.1.0"\n')
        source_bytes = {
            "Cargo.lock": b"version = 4\r\n",
            "LICENSE": b"license CRLF\r\nlicense LF\n",
            "THIRD_PARTY_NOTICES.md": b"notices CRLF\r\nnotices LF\n",
        }
        for path in source_bytes:
            (root / path).write_bytes(b"changed after validation\n")
        with mock.patch.object(
            release, "load_release_source_bytes", return_value=source_bytes
        ) as load_sources:
            archive = release.package_archive(
                binary=self.binary,
                target=release.TARGETS[0],
                source_commit=SOURCE_COMMIT,
                source_tree_state="clean_commit",
                rustc_version=RUSTC_VERSION,
                cargo_version=CARGO_VERSION,
                tag=None,
                output_dir=self.directory / "exact-byte-package",
                root=root,
            )
        load_sources.assert_called_once_with(SOURCE_COMMIT, "clean_commit", root)
        inspected = release.inspect_archive(
            archive, root, expected_release_source_bytes=source_bytes
        )
        archive_root = f"scrub-v0.1.0-{release.TARGETS[0]}"
        self.assertEqual(
            inspected.members[f"{archive_root}/LICENSE"], source_bytes["LICENSE"]
        )
        self.assertEqual(
            inspected.members[f"{archive_root}/THIRD_PARTY_NOTICES.md"],
            source_bytes["THIRD_PARTY_NOTICES.md"],
        )
        self.assertEqual(
            inspected.metadata["cargo_lock_sha256"],
            release.sha256_bytes(source_bytes["Cargo.lock"]),
        )

    def test_unknown_metadata_field_is_rejected(self) -> None:
        metadata = self.metadata()
        metadata["hostname"] = "builder"
        with self.assertRaisesRegex(release.ReleaseError, "unknown=.*hostname"):
            release.validate_metadata(metadata)

    def test_wrong_binary_digest_is_rejected(self) -> None:
        metadata = self.metadata()
        metadata["binary_sha256"] = "0" * 64
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        archive = self.directory / release.expected_archive_name("0.1.0", release.TARGETS[0])
        release.write_zip(archive, archive_root, members)
        with self.assertRaisesRegex(release.ReleaseError, "binary digest"):
            release.inspect_archive(archive)

    def test_wrong_cargo_lock_digest_is_rejected(self) -> None:
        metadata = self.metadata()
        metadata["cargo_lock_sha256"] = "0" * 64
        with self.assertRaisesRegex(release.ReleaseError, "Cargo.lock digest"):
            release.validate_metadata(metadata)

    def test_tag_metadata_cannot_describe_a_dirty_worktree(self) -> None:
        metadata = self.metadata()
        metadata["release_mode"] = "tag"
        metadata["release_tag"] = "v0.1.0"
        metadata["source_tree_state"] = "dirty_worktree"
        with self.assertRaisesRegex(release.ReleaseError, "dirty worktree"):
            release.validate_metadata(metadata)

    def test_absolute_path_leakage_is_detected(self) -> None:
        self.assertTrue(release.contains_absolute_path({"value": r"C:\Users\builder\work"}))
        self.assertTrue(release.contains_absolute_path({"value": "/home/runner/work"}))
        self.assertFalse(release.contains_absolute_path({"value": "scrub-v0.1.0"}))

    def test_zip_symlink_and_special_device_members_are_rejected(self) -> None:
        metadata = self.metadata()
        for label, member_type in (
            ("symlink", stat.S_IFLNK),
            ("character device", stat.S_IFCHR),
        ):
            with self.subTest(label=label):
                archive = self.write_zip_variant(metadata, binary_type=member_type)
                with self.assertRaisesRegex(release.ReleaseError, "ZIP member type"):
                    release.inspect_archive(archive)

    def test_noncanonical_zip_compression_and_create_system_are_rejected(self) -> None:
        metadata = self.metadata()
        cases = (
            {"file_compression": zipfile.ZIP_STORED},
            {"create_system": 0},
        )
        for arguments in cases:
            with self.subTest(arguments=arguments):
                archive = self.write_zip_variant(metadata, **arguments)
                with self.assertRaisesRegex(release.ReleaseError, "ZIP metadata"):
                    release.inspect_archive(archive)

    def test_noncanonical_zip_order_and_trailing_payload_are_rejected(self) -> None:
        metadata = self.metadata()
        archive = self.write_zip_variant(metadata, reverse_order=True)
        with self.assertRaisesRegex(release.ReleaseError, "reordered"):
            release.inspect_archive(archive)

        archive = self.write_zip_variant(metadata)
        archive.write_bytes(archive.read_bytes() + b"TRAILING-PAYLOAD")
        with self.assertRaisesRegex(release.ReleaseError, "ZIP end record"):
            release.inspect_archive(archive)

    def test_duplicate_release_metadata_keys_are_rejected(self) -> None:
        metadata = self.metadata()
        for key in ("source_commit", "binary_sha256", "rust_target", "package_version"):
            with self.subTest(key=key):
                canonical = release.json_bytes(metadata)
                encoded_value = json.dumps(metadata[key], ensure_ascii=False)
                line = f'  "{key}": {encoded_value},\n'.encode("utf-8")
                self.assertIn(line, canonical)
                duplicate = canonical.replace(line, line + line, 1)
                archive = self.write_zip_variant(metadata, metadata_bytes=duplicate)
                with self.assertRaisesRegex(release.ReleaseError, "duplicate object key"):
                    release.inspect_archive(archive)

    def test_unexpected_release_metadata_type_is_a_controlled_error(self) -> None:
        metadata = self.metadata()
        metadata["source_commit"] = 7
        archive = self.write_zip_variant(metadata)
        with self.assertRaisesRegex(release.ReleaseError, "source commit"):
            release.inspect_archive(archive)

    def test_archive_member_missing_and_extra_are_rejected(self) -> None:
        metadata = self.metadata()
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        for label, changed in (
            ("missing", {name: value for name, value in members.items() if not name.endswith("/LICENSE")}),
            ("extra", {**members, f"{archive_root}/unexpected.txt": (b"unexpected", 0o644)}),
        ):
            with self.subTest(label=label):
                archive = self.directory / release.expected_archive_name("0.1.0", release.TARGETS[0])
                release.write_zip(archive, archive_root, changed)
                with self.assertRaisesRegex(release.ReleaseError, "archive members disagree"):
                    release.inspect_archive(archive)

    def test_oversized_archive_member_is_rejected_before_decompression(self) -> None:
        target = "x86_64-unknown-linux-gnu"
        archive_root = f"scrub-v0.1.0-{target}"
        directory = tarfile.TarInfo(f"{archive_root}/")
        directory.type = tarfile.DIRTYPE
        directory.mode = 0o755
        directory.mtime = 0
        directory.uid = directory.gid = 0
        directory.uname = directory.gname = ""
        oversized = tarfile.TarInfo(f"{archive_root}/scrub")
        oversized.size = release.MAX_BINARY_BYTES + 1
        oversized.mode = 0o755
        oversized.mtime = 0
        oversized.uid = oversized.gid = 0
        oversized.uname = oversized.gname = ""
        tar_prefix = directory.tobuf(format=tarfile.USTAR_FORMAT) + oversized.tobuf(
            format=tarfile.USTAR_FORMAT
        )
        archive = self.directory / release.expected_archive_name("0.1.0", target)
        with archive.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
                compressed.write(tar_prefix)

        requested_sizes: list[int] = []
        original = release.CanonicalGzipReader.read_exact

        def observed_read_exact(reader: object, size: int) -> bytes:
            requested_sizes.append(size)
            return original(reader, size)

        with mock.patch.object(release.CanonicalGzipReader, "read_exact", observed_read_exact):
            with self.assertRaisesRegex(release.ReleaseError, "size bound"):
                release.inspect_archive(archive)
        self.assertEqual(requested_sizes, [tarfile.BLOCKSIZE, tarfile.BLOCKSIZE])

    def test_unix_executable_mode_loss_is_rejected(self) -> None:
        target = "x86_64-unknown-linux-gnu"
        metadata = self.metadata(target)
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        binary_name = f"{archive_root}/scrub"
        members[binary_name] = (members[binary_name][0], 0o644)
        archive = self.directory / release.expected_archive_name("0.1.0", target)
        release.write_tar_gz(archive, archive_root, members)
        with self.assertRaisesRegex(release.ReleaseError, "tar mode"):
            release.inspect_archive(archive)

    def test_hidden_pax_metadata_is_rejected(self) -> None:
        target = "x86_64-unknown-linux-gnu"
        metadata = self.metadata(target)
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        archive = self.directory / release.expected_archive_name("0.1.0", target)
        with archive.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
                with tarfile.open(
                    fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT
                ) as output:
                    directory = tarfile.TarInfo(f"{archive_root}/")
                    directory.type = tarfile.DIRTYPE
                    directory.mode = 0o755
                    directory.mtime = 0
                    directory.uid = directory.gid = 0
                    directory.uname = directory.gname = ""
                    output.addfile(directory)
                    for name in sorted(members):
                        data, mode = members[name]
                        info = tarfile.TarInfo(name)
                        info.size = len(data)
                        info.mode = mode
                        info.mtime = 0
                        info.uid = info.gid = 0
                        info.uname = info.gname = ""
                        if name.endswith("/LICENSE"):
                            info.pax_headers = {"comment": "hidden metadata"}
                        output.addfile(info, io.BytesIO(data))
        with self.assertRaisesRegex(release.ReleaseError, "unsupported member type"):
            release.inspect_archive(archive)

    def test_gzip_filename_and_trailing_payload_are_rejected(self) -> None:
        target = "x86_64-unknown-linux-gnu"
        metadata = self.metadata(target)
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        archive = self.directory / release.expected_archive_name("0.1.0", target)
        release.write_tar_gz(archive, archive_root, members)
        canonical = archive.read_bytes()
        tar_bytes = gzip.decompress(canonical)

        with archive.open("wb") as raw:
            with gzip.GzipFile(
                filename="host-path-leak.tar", mode="wb", fileobj=raw, mtime=0
            ) as compressed:
                compressed.write(tar_bytes)
        with self.assertRaisesRegex(release.ReleaseError, "gzip header"):
            release.inspect_archive(archive)

        for label, suffix in (
            ("trailing bytes", b"TRAILING-PAYLOAD"),
            ("concatenated gzip", gzip.compress(b"second member", mtime=0)),
        ):
            with self.subTest(label=label):
                archive.write_bytes(canonical + suffix)
                with self.assertRaisesRegex(release.ReleaseError, "trailing or concatenated"):
                    release.inspect_archive(archive)

    def test_nondeterministic_zip_metadata_is_rejected(self) -> None:
        metadata = self.metadata()
        archive_root, members = release.archive_members(metadata, self.binary.read_bytes())
        archive = self.directory / release.expected_archive_name("0.1.0", release.TARGETS[0])
        with zipfile.ZipFile(archive, "w") as output:
            directory = zipfile.ZipInfo(f"{archive_root}/", (1981, 1, 1, 0, 0, 0))
            directory.create_system = 3
            directory.external_attr = (stat.S_IFDIR | 0o755) << 16 | 0x10
            output.writestr(directory, b"")
            for name in sorted(members):
                data, mode = members[name]
                info = zipfile.ZipInfo(name, (1981, 1, 1, 0, 0, 0))
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | mode) << 16
                output.writestr(info, data)
        with self.assertRaisesRegex(release.ReleaseError, "not normalized"):
            release.inspect_archive(archive)

    def test_same_packaging_input_twice_has_same_archive_sha256(self) -> None:
        for target in (release.TARGETS[0], release.TARGETS[1]):
            with self.subTest(target=target):
                first = self.package(target, self.directory / "first")
                second = self.package(target, self.directory / "second")
                self.assertEqual(release.sha256_file(first), release.sha256_file(second))

    def test_package_metadata_target_disagreement_is_rejected(self) -> None:
        archive = self.package(release.TARGETS[0])
        with self.assertRaisesRegex(release.ReleaseError, "target disagrees"):
            release.verify_archive(archive, expected_target=release.TARGETS[1])

    def test_manifest_duplicate_missing_and_unknown_targets_are_rejected(self) -> None:
        packages, assembled = self.complete_release()
        document = json.loads((assembled / "release-manifest.json").read_text(encoding="utf-8"))
        cases = {}
        duplicate = copy.deepcopy(document)
        duplicate["artifacts"][1]["rust_target"] = release.TARGETS[0]
        cases["duplicate"] = duplicate
        missing = copy.deepcopy(document)
        missing["artifacts"].pop()
        cases["missing"] = missing
        unknown = copy.deepcopy(document)
        unknown["artifacts"][0]["rust_target"] = "unknown-target"
        cases["unknown"] = unknown
        for label, candidate in cases.items():
            with self.subTest(label=label), self.assertRaises(release.ReleaseError):
                release.validate_manifest(candidate)
        self.assertTrue(packages.is_dir())

    def test_missing_archive_cannot_form_complete_release(self) -> None:
        packages = self.directory / "packages"
        for target in release.TARGETS[:-1]:
            self.package(target, packages)
        with self.assertRaisesRegex(release.ReleaseError, "not exact"):
            release.build_release_manifest(
                input_dir=packages,
                source_commit=SOURCE_COMMIT,
                source_tree_state="clean_commit",
                tag=None,
            )

    def test_wrong_archive_digest_is_rejected(self) -> None:
        packages, assembled = self.complete_release()
        document = json.loads((assembled / "release-manifest.json").read_text(encoding="utf-8"))
        document["artifacts"][0]["archive_sha256"] = "0" * 64
        self.rewrite_manifest_and_checksums(packages, assembled, document)
        with self.assertRaisesRegex(release.ReleaseError, "archive digest disagrees"):
            release.verify_release_output(assembled, archive_dir=packages)

    def test_sha256sums_drift_is_rejected(self) -> None:
        packages, assembled = self.complete_release()
        sums = assembled / "SHA256SUMS"
        lines = sums.read_text(encoding="ascii").splitlines()
        lines[0] = "0" * 64 + lines[0][64:]
        sums.write_text("\n".join(lines) + "\n", encoding="ascii", newline="\n")
        with self.assertRaisesRegex(release.ReleaseError, "SHA256SUMS .*digest"):
            release.verify_release_output(assembled, archive_dir=packages)

    def test_verify_release_rejects_every_extra_archive(self) -> None:
        for name in (
            "unexpected-fifth.zip",
            "unexpected-fifth.tar.gz",
            "scrub-v0.0.9-x86_64-pc-windows-msvc.zip",
        ):
            with self.subTest(name=name):
                packages, assembled = self.complete_release()
                extra = packages / name
                extra.write_bytes(b"unexpected archive")
                with self.assertRaisesRegex(release.ReleaseError, "exact four-target set"):
                    release.verify_release_output(assembled, archive_dir=packages)
                extra.unlink()

    def test_duplicate_release_manifest_key_is_rejected(self) -> None:
        packages, assembled = self.complete_release()
        manifest = assembled / "release-manifest.json"
        value = manifest.read_bytes()
        line = f'  "source_commit": "{SOURCE_COMMIT}",\n'.encode("ascii")
        self.assertIn(line, value)
        manifest.write_bytes(value.replace(line, line + line, 1))
        with self.assertRaisesRegex(release.ReleaseError, "duplicate object key"):
            release.verify_release_output(assembled, archive_dir=packages)

        document = json.loads(value)
        document["source_commit"] = 7
        manifest.write_bytes(release.json_bytes(document))
        with self.assertRaisesRegex(release.ReleaseError, "source commit"):
            release.verify_release_output(assembled, archive_dir=packages)

    def test_malformed_archives_use_the_controlled_cli_error_path(self) -> None:
        cases = (
            (
                self.directory / release.expected_archive_name("0.1.0", release.TARGETS[0]),
                b"not a ZIP archive",
            ),
            (
                self.directory / release.expected_archive_name("0.1.0", release.TARGETS[1]),
                release.CANONICAL_GZIP_HEADER + b"not deflate",
            ),
        )
        for archive, value in cases:
            with self.subTest(archive=archive.name):
                archive.write_bytes(value)
                with self.assertRaises(release.ReleaseError):
                    release.inspect_archive(archive)
                stderr = io.StringIO()
                with mock.patch("sys.stderr", stderr):
                    result = release.main(
                        ["verify-package", "--archive", str(archive), "--preflight"]
                    )
                self.assertEqual(result, 2)
                self.assertIn("release contract failed:", stderr.getvalue())
                self.assertNotIn("Traceback", stderr.getvalue())

    def test_manifest_source_commit_disagreement_is_rejected(self) -> None:
        packages, assembled = self.complete_release()
        document = json.loads((assembled / "release-manifest.json").read_text(encoding="utf-8"))
        document["source_commit"] = "b" * 40
        self.rewrite_manifest_and_checksums(packages, assembled, document)
        with self.assertRaisesRegex(release.ReleaseError, "source commit disagrees"):
            release.verify_release_output(assembled, archive_dir=packages)

    def test_checksums_are_sorted_lf_only_and_exclude_themselves(self) -> None:
        packages, assembled = self.complete_release()
        value = (assembled / "SHA256SUMS").read_bytes()
        parsed = release.parse_checksums(value)
        self.assertNotIn(b"\r", value)
        self.assertNotIn("SHA256SUMS", parsed)
        self.assertEqual(list(parsed), sorted(parsed))
        self.assertEqual(len(parsed), 5)
        release.verify_release_output(assembled, archive_dir=packages)

    def test_strict_schemas_accept_generated_documents(self) -> None:
        try:
            from jsonschema import Draft202012Validator
        except ImportError:
            self.skipTest("jsonschema is unavailable in this interpreter")
        packages, assembled = self.complete_release()
        manifest = json.loads((assembled / "release-manifest.json").read_text(encoding="utf-8"))
        metadata = release.inspect_archive(next(packages.glob("*.zip"))).metadata
        for schema_name, document in (
            ("release-artifact-0.1.schema.json", metadata),
            ("release-manifest-0.1.schema.json", manifest),
        ):
            schema = json.loads((ROOT / "schemas" / schema_name).read_text(encoding="utf-8"))
            Draft202012Validator.check_schema(schema)
            Draft202012Validator(schema).validate(document)

    def test_release_workflow_uses_exact_matrix_and_immutable_action_pins(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )
        uses = re.findall(r"^\s*uses:\s*([^\s#]+)", workflow, flags=re.MULTILINE)
        self.assertTrue(uses)
        for action in uses:
            with self.subTest(action=action):
                self.assertRegex(action, r"^[^@]+@[0-9a-f]{40}$")
        self.assertIn(
            "actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6", uses
        )
        self.assertNotIn("-latest", workflow)
        for runner in ("windows-2025", "ubuntu-24.04", "macos-15", "macos-15-intel"):
            self.assertIn(f"runner: {runner}", workflow)
        for target in release.TARGETS:
            self.assertIn(f"target: {target}", workflow)
        self.assertIn("subject-path: ${{ steps.package.outputs.archive_path }}", workflow)
        self.assertNotIn("artifact-metadata: write", workflow)
        self.assertNotIn("pull_request_target", workflow)

    def test_release_workflow_stops_at_a_draft_and_verifies_existing_tag(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("--draft", workflow)
        self.assertIn("--verify-tag", workflow)
        self.assertIn('test "$(git rev-list -n 1', workflow)
        self.assertNotRegex(workflow, r"gh release (?:edit|create)[^\n]*--draft=false")
        self.assertNotIn("gh release create --generate-notes", workflow)


if __name__ == "__main__":
    unittest.main()
