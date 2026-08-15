#!/usr/bin/env python3
"""Deterministic scrub release packaging and semantic verification."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import stat
import struct
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from typing import Any, Iterable
import zipfile
import zlib


TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))
from third_party_licenses import (  # noqa: E402
    LicenseBundleError,
    verify_reviewed_bundle,
)


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = "0.1"
TOOLCHAIN_VERSION = "1.97.1"
TARGETS = (
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
)
TARGET_CONFIG = {
    "x86_64-pc-windows-msvc": ("scrub.exe", ".zip"),
    "x86_64-unknown-linux-gnu": ("scrub", ".tar.gz"),
    "aarch64-apple-darwin": ("scrub", ".tar.gz"),
    "x86_64-apple-darwin": ("scrub", ".tar.gz"),
}
RELEASE_SOURCE_BYTE_PATHS = (
    "Cargo.lock",
    "LICENSE",
    "THIRD_PARTY_LICENSES.txt",
    "THIRD_PARTY_NOTICES.md",
)
SEMVER = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
SEMVER_RE = re.compile(rf"^{SEMVER}$")
TAG_RE = re.compile(rf"^v{SEMVER}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
RUSTC_RE = re.compile(r"^rustc 1\.97\.1 \([^)]+\)$")
CARGO_RE = re.compile(r"^cargo 1\.97\.1 \([^)]+\)$")
LIMITATIONS = (
    "Deterministic archive construction does not establish bit-for-bit reproducible compiler output.",
    "GitHub artifact attestations do not provide Apple Developer ID, Apple notarization, or Windows Authenticode signing.",
)
SIGNING_STATUS = {
    "apple_developer_id": "not_provided",
    "apple_notarization": "not_provided",
    "windows_authenticode": "not_provided",
}
MAX_ARCHIVE_BYTES = 256 * 1024 * 1024
MAX_BINARY_BYTES = 128 * 1024 * 1024
MAX_METADATA_BYTES = 64 * 1024
MAX_SUPPORT_FILE_BYTES = 4 * 1024 * 1024
MAX_MEMBER_NAME_BYTES = 256
CANONICAL_GZIP_HEADER = b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x02\xff"
ZIP_END_RECORD = struct.Struct("<4s4H2LH")
ZIP_LOCAL_HEADER = struct.Struct("<4s5H3L2H")
METADATA_KEYS = {
    "schema_version",
    "release_mode",
    "release_tag",
    "package_name",
    "package_version",
    "source_commit",
    "source_tree_state",
    "rust_target",
    "binary_filename",
    "binary_sha256",
    "cargo_lock_sha256",
    "rustc_version",
    "cargo_version",
    "release_profile",
    "platform_vendor_signing",
    "limitations",
}
MANIFEST_KEYS = {
    "schema_version",
    "release_mode",
    "package_name",
    "package_version",
    "release_tag",
    "source_commit",
    "source_tree_state",
    "expected_targets",
    "artifacts",
}
MANIFEST_ROW_KEYS = {
    "archive_filename",
    "archive_sha256",
    "rust_target",
    "binary_sha256",
    "release_metadata_sha256",
    "packaging_schema_version",
}


class ReleaseError(ValueError):
    """A release input violates the repository contract."""


def validate_third_party_license_bundle(root: Path | None = None) -> None:
    if root is None:
        root = ROOT
    try:
        verify_reviewed_bundle(root)
    except LicenseBundleError as error:
        raise ReleaseError(f"third-party license bundle is invalid: {error}") from error


@dataclass(frozen=True)
class InspectedArchive:
    path: Path
    metadata: dict[str, Any]
    metadata_bytes: bytes
    binary_bytes: bytes
    binary_mode: int
    members: dict[str, bytes]
    member_modes: dict[str, int]


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def committed_release_source_bytes(
    source_commit: str, path: str, root: Path = ROOT
) -> bytes:
    if path not in RELEASE_SOURCE_BYTE_PATHS:
        raise ReleaseError(f"unknown exact-byte release source path: {path}")
    try:
        completed = subprocess.run(
            ["git", "cat-file", "blob", f"{source_commit}:{path}"],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        raise ReleaseError(
            f"cannot read {path} from source commit {source_commit}"
        ) from error
    return completed.stdout


def working_release_source_bytes(root: Path = ROOT) -> dict[str, bytes]:
    values: dict[str, bytes] = {}
    for path in RELEASE_SOURCE_BYTE_PATHS:
        try:
            values[path] = (root / path).read_bytes()
        except OSError as error:
            raise ReleaseError(f"cannot read exact-byte release source path: {path}") from error
    return values


def validate_release_source_byte_set(values: dict[str, bytes]) -> None:
    if set(values) != set(RELEASE_SOURCE_BYTE_PATHS) or any(
        not isinstance(value, bytes) for value in values.values()
    ):
        raise ReleaseError("exact-byte release source set is incomplete or invalid")


def load_release_source_bytes(
    source_commit: str, source_tree_state: str, root: Path = ROOT
) -> dict[str, bytes]:
    working = working_release_source_bytes(root)
    if source_tree_state == "clean_commit":
        for path in RELEASE_SOURCE_BYTE_PATHS:
            committed = committed_release_source_bytes(source_commit, path, root)
            if working[path] != committed:
                raise ReleaseError(
                    f"clean-commit {path} bytes disagree with the source commit blob"
                )
    return working


def json_bytes(document: dict[str, Any]) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode(
        "utf-8"
    )


def read_json_bytes(value: bytes, label: str) -> dict[str, Any]:
    def strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        document: dict[str, Any] = {}
        for key, member in pairs:
            if key in document:
                raise ReleaseError(f"{label} contains duplicate object key: {key!r}")
            document[key] = member
        return document

    try:
        document = json.loads(value.decode("utf-8"), object_pairs_hook=strict_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ReleaseError(f"{label} is not valid UTF-8 JSON: {error}") from error
    if not isinstance(document, dict):
        raise ReleaseError(f"{label} must contain a JSON object")
    return document


def manifest_versions(root: Path = ROOT) -> tuple[str, str]:
    def package_version(relative: str) -> str:
        with (root / relative).open("rb") as handle:
            document = tomllib.load(handle)
        try:
            version = document["package"]["version"]
        except (KeyError, TypeError) as error:
            raise ReleaseError(f"{relative} has no literal package version") from error
        if not isinstance(version, str) or not SEMVER_RE.fullmatch(version):
            raise ReleaseError(f"{relative} has an unsupported package version: {version!r}")
        return version

    return (
        package_version("crates/scrub/Cargo.toml"),
        package_version("crates/scrub-report/Cargo.toml"),
    )


def validate_version_contract(tag: str | None, root: Path = ROOT) -> str:
    scrub_version, report_version = manifest_versions(root)
    if scrub_version != report_version:
        raise ReleaseError(
            "scrub and scrub-report package versions are ambiguous for release: "
            f"{scrub_version} != {report_version}"
        )
    if tag is not None:
        if not isinstance(tag, str):
            raise ReleaseError("release tag must be a string or null")
        if not TAG_RE.fullmatch(tag):
            raise ReleaseError(f"malformed release tag: {tag!r}")
        expected = f"v{scrub_version}"
        if tag != expected:
            raise ReleaseError(f"release tag {tag!r} does not match package version {expected!r}")
    return scrub_version


def validate_commit(value: str) -> None:
    if not isinstance(value, str) or not COMMIT_RE.fullmatch(value):
        raise ReleaseError("source commit must be exactly 40 lowercase hexadecimal characters")


def release_mode(tag: str | None) -> str:
    return "tag" if tag is not None else "preflight"


def ensure_exact_keys(document: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(document)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        raise ReleaseError(f"{label} fields disagree; missing={missing}, unknown={unknown}")


def contains_absolute_path(value: Any) -> bool:
    if isinstance(value, dict):
        return any(contains_absolute_path(member) for member in value.values())
    if isinstance(value, list):
        return any(contains_absolute_path(member) for member in value)
    if not isinstance(value, str) or not value:
        return False
    if PureWindowsPath(value).is_absolute() or PurePosixPath(value).is_absolute():
        return True
    return bool(re.search(r"(?:^|\s)[A-Za-z]:[\\/]", value) or value.startswith("\\\\"))


def validate_metadata(
    document: dict[str, Any],
    root: Path = ROOT,
    release_source_bytes: dict[str, bytes] | None = None,
) -> None:
    if not isinstance(document, dict):
        raise ReleaseError("release metadata must be an object")
    ensure_exact_keys(document, METADATA_KEYS, "release metadata")
    if document["schema_version"] != SCHEMA_VERSION:
        raise ReleaseError("unknown release metadata schema version")
    mode = document["release_mode"]
    tag = document["release_tag"]
    if not isinstance(mode, str) or mode not in {"preflight", "tag"}:
        raise ReleaseError("release metadata has an unknown mode")
    if tag is not None and not isinstance(tag, str):
        raise ReleaseError("release metadata tag must be a string or null")
    if (mode == "tag") != (tag is not None):
        raise ReleaseError("release mode and release tag disagree")
    version = validate_version_contract(tag, root)
    if (
        not isinstance(document["package_name"], str)
        or not isinstance(document["package_version"], str)
        or document["package_name"] != "scrub"
        or document["package_version"] != version
    ):
        raise ReleaseError("release metadata package identity disagrees with Cargo manifests")
    validate_commit(document["source_commit"])
    if not isinstance(document["source_tree_state"], str) or document[
        "source_tree_state"
    ] not in {"clean_commit", "dirty_worktree"}:
        raise ReleaseError("release metadata has an unknown source tree state")
    if mode == "tag" and document["source_tree_state"] != "clean_commit":
        raise ReleaseError("tag release metadata cannot describe a dirty worktree")
    target = document["rust_target"]
    if not isinstance(target, str) or target not in TARGET_CONFIG:
        raise ReleaseError(f"unknown release target: {target!r}")
    expected_binary, _extension = TARGET_CONFIG[target]
    if not isinstance(document["binary_filename"], str) or document[
        "binary_filename"
    ] != expected_binary:
        raise ReleaseError("release metadata binary filename disagrees with target")
    for field in ("binary_sha256", "cargo_lock_sha256"):
        if not isinstance(document[field], str) or not SHA256_RE.fullmatch(document[field]):
            raise ReleaseError(f"release metadata has malformed {field}")
    if release_source_bytes is None:
        release_source_bytes = working_release_source_bytes(root)
    validate_release_source_byte_set(release_source_bytes)
    if document["cargo_lock_sha256"] != sha256_bytes(release_source_bytes["Cargo.lock"]):
        raise ReleaseError("release metadata Cargo.lock digest disagrees with repository lock")
    if not isinstance(document["rustc_version"], str) or not RUSTC_RE.fullmatch(
        document["rustc_version"]
    ):
        raise ReleaseError(f"release metadata must record rustc {TOOLCHAIN_VERSION}")
    if not isinstance(document["cargo_version"], str) or not CARGO_RE.fullmatch(
        document["cargo_version"]
    ):
        raise ReleaseError(f"release metadata must record cargo {TOOLCHAIN_VERSION}")
    if document["release_profile"] != "cargo --locked --release":
        raise ReleaseError("release metadata has an unknown release profile")
    if document["platform_vendor_signing"] != SIGNING_STATUS:
        raise ReleaseError("release metadata has an impossible platform-vendor signing state")
    if document["limitations"] != list(LIMITATIONS):
        raise ReleaseError("release metadata limitations are missing or changed")
    if contains_absolute_path(document):
        raise ReleaseError("release metadata contains an absolute local path")


def build_metadata(
    *,
    binary: Path,
    target: str,
    source_commit: str,
    source_tree_state: str,
    rustc_version: str,
    cargo_version: str,
    tag: str | None,
    root: Path = ROOT,
    release_source_bytes: dict[str, bytes] | None = None,
) -> dict[str, Any]:
    if target not in TARGET_CONFIG:
        raise ReleaseError(f"unknown release target: {target!r}")
    if not binary.is_file():
        raise ReleaseError(f"release binary does not exist: {binary}")
    validate_commit(source_commit)
    version = validate_version_contract(tag, root)
    if release_source_bytes is None:
        release_source_bytes = load_release_source_bytes(
            source_commit, source_tree_state, root
        )
    validate_release_source_byte_set(release_source_bytes)
    binary_name, _extension = TARGET_CONFIG[target]
    document = {
        "schema_version": SCHEMA_VERSION,
        "release_mode": release_mode(tag),
        "release_tag": tag,
        "package_name": "scrub",
        "package_version": version,
        "source_commit": source_commit,
        "source_tree_state": source_tree_state,
        "rust_target": target,
        "binary_filename": binary_name,
        "binary_sha256": sha256_file(binary),
        "cargo_lock_sha256": sha256_bytes(release_source_bytes["Cargo.lock"]),
        "rustc_version": rustc_version,
        "cargo_version": cargo_version,
        "release_profile": "cargo --locked --release",
        "platform_vendor_signing": dict(SIGNING_STATUS),
        "limitations": list(LIMITATIONS),
    }
    validate_metadata(document, root, release_source_bytes)
    return document


def expected_archive_name(version: str, target: str) -> str:
    _binary, extension = TARGET_CONFIG[target]
    return f"scrub-v{version}-{target}{extension}"


def archive_members(
    metadata: dict[str, Any],
    binary_bytes: bytes,
    root: Path = ROOT,
    release_source_bytes: dict[str, bytes] | None = None,
) -> tuple[str, dict[str, tuple[bytes, int]]]:
    if release_source_bytes is None:
        release_source_bytes = working_release_source_bytes(root)
    validate_release_source_byte_set(release_source_bytes)
    archive_root = f"scrub-v{metadata['package_version']}-{metadata['rust_target']}"
    members = {
        f"{archive_root}/{metadata['binary_filename']}": (binary_bytes, 0o755),
        f"{archive_root}/LICENSE": (release_source_bytes["LICENSE"], 0o644),
        f"{archive_root}/THIRD_PARTY_LICENSES.txt": (
            release_source_bytes["THIRD_PARTY_LICENSES.txt"],
            0o644,
        ),
        f"{archive_root}/THIRD_PARTY_NOTICES.md": (
            release_source_bytes["THIRD_PARTY_NOTICES.md"],
            0o644,
        ),
        f"{archive_root}/RELEASE-METADATA.json": (json_bytes(metadata), 0o644),
    }
    return archive_root, members


def write_zip(path: Path, archive_root: str, members: dict[str, tuple[bytes, int]]) -> None:
    with zipfile.ZipFile(
        path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=True
    ) as archive:
        directory = zipfile.ZipInfo(f"{archive_root}/", (1980, 1, 1, 0, 0, 0))
        directory.create_system = 3
        directory.compress_type = zipfile.ZIP_STORED
        directory.external_attr = (stat.S_IFDIR | 0o755) << 16 | 0x10
        archive.writestr(directory, b"")
        for name in sorted(members):
            data, mode = members[name]
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (stat.S_IFREG | mode) << 16
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


def write_tar_gz(path: Path, archive_root: str, members: dict[str, tuple[bytes, int]]) -> None:
    with path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", compresslevel=9, fileobj=raw, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as archive:
                directory = tarfile.TarInfo(f"{archive_root}/")
                directory.type = tarfile.DIRTYPE
                directory.mode = 0o755
                directory.mtime = 0
                directory.uid = directory.gid = 0
                directory.uname = directory.gname = ""
                archive.addfile(directory)
                for name in sorted(members):
                    data, mode = members[name]
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = mode
                    info.mtime = 0
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    archive.addfile(info, io.BytesIO(data))


def package_archive(
    *,
    binary: Path,
    target: str,
    source_commit: str,
    source_tree_state: str,
    rustc_version: str,
    cargo_version: str,
    tag: str | None,
    output_dir: Path,
    root: Path = ROOT,
) -> Path:
    release_source_bytes = load_release_source_bytes(
        source_commit, source_tree_state, root
    )
    metadata = build_metadata(
        binary=binary,
        target=target,
        source_commit=source_commit,
        source_tree_state=source_tree_state,
        rustc_version=rustc_version,
        cargo_version=cargo_version,
        tag=tag,
        root=root,
        release_source_bytes=release_source_bytes,
    )
    archive_root, members = archive_members(
        metadata, binary.read_bytes(), root, release_source_bytes
    )
    output_dir.mkdir(parents=True, exist_ok=True)
    output = output_dir / expected_archive_name(metadata["package_version"], target)
    temporary = output.with_name(output.name + ".tmp")
    if target == "x86_64-pc-windows-msvc":
        write_zip(temporary, archive_root, members)
    else:
        write_tar_gz(temporary, archive_root, members)
    temporary.replace(output)
    inspect_archive(
        output, root=root, expected_release_source_bytes=release_source_bytes
    )
    return output


def expected_member_names(metadata: dict[str, Any]) -> tuple[str, list[str]]:
    archive_root = f"scrub-v{metadata['package_version']}-{metadata['rust_target']}"
    names = [
        f"{archive_root}/",
        f"{archive_root}/{metadata['binary_filename']}",
        f"{archive_root}/LICENSE",
        f"{archive_root}/RELEASE-METADATA.json",
        f"{archive_root}/THIRD_PARTY_LICENSES.txt",
        f"{archive_root}/THIRD_PARTY_NOTICES.md",
    ]
    return archive_root, sorted(names)


def canonical_archive_layout(path: Path, root: Path = ROOT) -> tuple[str, str, list[str]]:
    version = validate_version_contract(None, root)
    targets = [target for target in TARGETS if path.name == expected_archive_name(version, target)]
    if len(targets) != 1:
        raise ReleaseError(f"archive filename is not canonical for scrub {version}: {path.name!r}")
    target = targets[0]
    binary_name, _extension = TARGET_CONFIG[target]
    archive_root = f"scrub-v{version}-{target}"
    files = [
        f"{archive_root}/{binary_name}",
        f"{archive_root}/LICENSE",
        f"{archive_root}/THIRD_PARTY_LICENSES.txt",
        f"{archive_root}/THIRD_PARTY_NOTICES.md",
        f"{archive_root}/RELEASE-METADATA.json",
    ]
    return target, archive_root, [f"{archive_root}/", *sorted(files)]


def validate_member_size(name: str, size: int, is_directory: bool = False) -> None:
    if len(name.encode("utf-8")) > MAX_MEMBER_NAME_BYTES:
        raise ReleaseError("archive member name exceeds the v0.1 bound")
    if is_directory:
        if size != 0:
            raise ReleaseError("archive directory member must be empty")
        return
    if name.endswith("/RELEASE-METADATA.json"):
        limit = MAX_METADATA_BYTES
    elif (
        name.endswith("/LICENSE")
        or name.endswith("/THIRD_PARTY_LICENSES.txt")
        or name.endswith("/THIRD_PARTY_NOTICES.md")
    ):
        limit = MAX_SUPPORT_FILE_BYTES
    else:
        limit = MAX_BINARY_BYTES
    if size < 0 or size > limit:
        raise ReleaseError(f"archive member exceeds the v0.1 size bound: {name}")


def inspect_zip(
    path: Path, expected_names: list[str]
) -> tuple[dict[str, bytes], dict[str, int]]:
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if names != expected_names:
            raise ReleaseError(
                f"archive members disagree or are reordered; expected={expected_names}, actual={names}"
            )
        if archive.comment:
            raise ReleaseError("ZIP archive comment is not normalized")
        contents: dict[str, bytes] = {}
        modes: dict[str, int] = {}
        for info in infos:
            validate_member_size(info.filename, info.file_size, info.is_dir())
            is_directory = info.filename.endswith("/")
            expected_mode = 0o755 if is_directory or info.filename.endswith("/scrub.exe") else 0o644
            expected_type = stat.S_IFDIR if is_directory else stat.S_IFREG
            expected_external = (expected_type | expected_mode) << 16
            if is_directory:
                expected_external |= 0x10
            actual_type = stat.S_IFMT(info.external_attr >> 16)
            if actual_type != expected_type:
                raise ReleaseError(f"ZIP member type is not canonical for {info.filename}")
            if (
                info.create_system != 3
                or info.create_version != 20
                or info.extract_version != 20
                or info.reserved != 0
                or info.flag_bits != 0
                or info.volume != 0
                or info.internal_attr != 0
                or info.external_attr != expected_external
                or info.compress_type
                != (zipfile.ZIP_STORED if is_directory else zipfile.ZIP_DEFLATED)
                or info.date_time != (1980, 1, 1, 0, 0, 0)
                or info.extra
                or info.comment
            ):
                raise ReleaseError(f"ZIP metadata is not normalized for {info.filename}")
            mode = stat.S_IMODE(info.external_attr >> 16)
            if mode != expected_mode:
                raise ReleaseError(f"ZIP mode is not normalized for {info.filename}")
            modes[info.filename] = mode
            if not is_directory:
                contents[info.filename] = archive.read(info)

        with path.open("rb") as raw:
            raw.seek(0, os.SEEK_END)
            archive_size = raw.tell()
            if archive_size < ZIP_END_RECORD.size:
                raise ReleaseError("ZIP end record is missing")
            raw.seek(-ZIP_END_RECORD.size, os.SEEK_END)
            end_record = ZIP_END_RECORD.unpack(raw.read(ZIP_END_RECORD.size))
            (
                signature,
                disk_number,
                central_disk,
                disk_entries,
                total_entries,
                central_size,
                central_offset,
                comment_size,
            ) = end_record
            if (
                signature != b"PK\x05\x06"
                or disk_number != 0
                or central_disk != 0
                or disk_entries != len(expected_names)
                or total_entries != len(expected_names)
                or comment_size != 0
                or central_offset != archive.start_dir
                or central_offset + central_size != archive_size - ZIP_END_RECORD.size
            ):
                raise ReleaseError("ZIP end record is not canonical")

            next_offset = 0
            for info in infos:
                if info.header_offset != next_offset:
                    raise ReleaseError("ZIP local members are not contiguous and canonical")
                raw.seek(info.header_offset)
                local = raw.read(ZIP_LOCAL_HEADER.size)
                if len(local) != ZIP_LOCAL_HEADER.size:
                    raise ReleaseError("ZIP local header is truncated")
                (
                    local_signature,
                    extract_version,
                    flag_bits,
                    compress_type,
                    modified_time,
                    modified_date,
                    crc32,
                    compressed_size,
                    file_size,
                    filename_size,
                    extra_size,
                ) = ZIP_LOCAL_HEADER.unpack(local)
                filename = raw.read(filename_size)
                extra = raw.read(extra_size)
                if (
                    local_signature != b"PK\x03\x04"
                    or extract_version != 20
                    or flag_bits != 0
                    or compress_type != info.compress_type
                    or modified_time != 0
                    or modified_date != 33
                    or crc32 != info.CRC
                    or compressed_size != info.compress_size
                    or file_size != info.file_size
                    or filename != info.filename.encode("ascii")
                    or extra
                ):
                    raise ReleaseError(f"ZIP local header is not canonical for {info.filename}")
                next_offset = (
                    info.header_offset
                    + ZIP_LOCAL_HEADER.size
                    + filename_size
                    + extra_size
                    + info.compress_size
                )
            if next_offset != central_offset:
                raise ReleaseError("ZIP local data does not end at the central directory")
        return contents, modes


class CanonicalGzipReader:
    """Bounded raw-DEFLATE reader for scrub's one canonical gzip member."""

    def __init__(self, value: bytes):
        if not value.startswith(CANONICAL_GZIP_HEADER):
            raise ReleaseError("gzip header is not canonical")
        self._value = value[10:]
        self._position = 0
        self._pending = b""
        self._decompressor = zlib.decompressobj(-zlib.MAX_WBITS)
        self._crc32 = 0
        self._size = 0
        self._finished = False

    def _read_some(self, maximum: int) -> bytes:
        while not self._finished:
            if self._pending:
                compressed = self._pending
                self._pending = b""
            else:
                if self._position >= len(self._value):
                    raise ReleaseError("gzip compressed stream is truncated")
                end = min(self._position + 64 * 1024, len(self._value))
                compressed = self._value[self._position : end]
                self._position = end
            try:
                result = self._decompressor.decompress(compressed, maximum)
            except zlib.error as error:
                raise ReleaseError(f"gzip compressed stream is malformed: {error}") from error
            self._pending = self._decompressor.unconsumed_tail
            self._crc32 = zlib.crc32(result, self._crc32)
            self._size = (self._size + len(result)) & 0xFFFFFFFF
            if self._decompressor.eof:
                trailer_and_after = self._decompressor.unused_data + self._value[self._position :]
                if len(trailer_and_after) != 8:
                    raise ReleaseError("gzip archive has trailing or concatenated data")
                expected_crc32, expected_size = struct.unpack("<II", trailer_and_after)
                if expected_crc32 != self._crc32 or expected_size != self._size:
                    raise ReleaseError("gzip trailer disagrees with decompressed tar bytes")
                self._finished = True
                self._pending = b""
            if result:
                return result
        return b""

    def read_exact(self, size: int) -> bytes:
        result = bytearray()
        while len(result) < size:
            member = self._read_some(size - len(result))
            if not member:
                raise ReleaseError("gzip tar stream is truncated")
            result.extend(member)
        return bytes(result)

    def finish(self) -> None:
        if self._read_some(1):
            raise ReleaseError("gzip tar stream contains data after the canonical tar end")
        if not self._finished:
            raise ReleaseError("gzip compressed stream did not terminate")


def inspect_tar_gz(
    path: Path, expected_names: list[str]
) -> tuple[dict[str, bytes], dict[str, int]]:
    reader = CanonicalGzipReader(path.read_bytes())
    contents: dict[str, bytes] = {}
    modes: dict[str, int] = {}
    tar_offset = 0
    for expected_name in expected_names:
        header = reader.read_exact(tarfile.BLOCKSIZE)
        tar_offset += tarfile.BLOCKSIZE
        if header == tarfile.NUL * tarfile.BLOCKSIZE:
            raise ReleaseError("tar archive ended before all canonical members")
        try:
            info = tarfile.TarInfo.frombuf(header, "utf-8", "strict")
        except (tarfile.TarError, UnicodeError) as error:
            raise ReleaseError(f"tar member header is malformed: {error}") from error
        is_directory = info.type == tarfile.DIRTYPE
        is_regular = info.type == tarfile.REGTYPE
        normalized_name = info.name + ("/" if is_directory else "")

        if not (is_directory or is_regular):
            raise ReleaseError(f"tar archive has unsupported member type: {normalized_name}")
        validate_member_size(normalized_name, info.size, is_directory)
        if normalized_name != expected_name:
            raise ReleaseError(
                f"tar member or order disagrees; expected={expected_name!r}, actual={normalized_name!r}"
            )
        try:
            canonical_header = info.tobuf(
                format=tarfile.USTAR_FORMAT, encoding="utf-8", errors="strict"
            )
        except (ValueError, UnicodeError) as error:
            raise ReleaseError(f"tar member is not canonical USTAR: {normalized_name}") from error
        if header != canonical_header or info.pax_headers:
            raise ReleaseError(f"tar header is not canonical USTAR for {normalized_name}")
        if any((info.mtime != 0, info.uid != 0, info.gid != 0, info.uname, info.gname)):
            raise ReleaseError(f"tar metadata is not normalized for {normalized_name}")
        expected_mode = 0o755 if is_directory or normalized_name.endswith("/scrub") else 0o644
        if info.mode != expected_mode:
            raise ReleaseError(f"tar mode is not normalized for {normalized_name}")
        modes[normalized_name] = info.mode

        if is_regular:
            contents[normalized_name] = reader.read_exact(info.size)
            tar_offset += info.size
        padding_size = (-info.size) % tarfile.BLOCKSIZE
        if padding_size:
            padding = reader.read_exact(padding_size)
            tar_offset += padding_size
            if any(padding):
                raise ReleaseError(f"tar payload padding is not canonical for {normalized_name}")

    canonical_size = (
        (tar_offset + 2 * tarfile.BLOCKSIZE + tarfile.RECORDSIZE - 1)
        // tarfile.RECORDSIZE
        * tarfile.RECORDSIZE
    )
    ending = reader.read_exact(canonical_size - tar_offset)
    if any(ending):
        raise ReleaseError("tar end records or record padding are not canonical")
    reader.finish()
    return contents, modes


def inspect_archive(
    path: Path,
    root: Path = ROOT,
    expected_release_source_bytes: dict[str, bytes] | None = None,
) -> InspectedArchive:
    if path.stat().st_size > MAX_ARCHIVE_BYTES:
        raise ReleaseError("release archive exceeds the v0.1 size bound")
    _target, archive_root, canonical_names = canonical_archive_layout(path, root)
    try:
        if path.name.endswith(".tar.gz"):
            contents, modes = inspect_tar_gz(path, canonical_names)
        elif path.suffix == ".zip":
            contents, modes = inspect_zip(path, canonical_names)
        else:
            raise ReleaseError(f"unsupported release archive format: {path.name}")
    except ReleaseError:
        raise
    except (zipfile.BadZipFile, tarfile.TarError, zlib.error, UnicodeError) as error:
        raise ReleaseError(f"release archive is malformed: {error}") from error
    metadata_names = [name for name in contents if name.endswith("/RELEASE-METADATA.json")]
    if len(metadata_names) != 1:
        raise ReleaseError("archive must contain exactly one RELEASE-METADATA.json")
    metadata_name = metadata_names[0]
    metadata_bytes = contents[metadata_name]
    metadata = read_json_bytes(metadata_bytes, "RELEASE-METADATA.json")
    if expected_release_source_bytes is None:
        expected_release_source_bytes = working_release_source_bytes(root)
    validate_release_source_byte_set(expected_release_source_bytes)
    validate_metadata(metadata, root, expected_release_source_bytes)
    metadata_archive_root, expected = expected_member_names(metadata)
    if metadata_archive_root != archive_root:
        raise ReleaseError("archive root disagrees with its canonical filename")
    actual = sorted([*contents, *(name for name in modes if name.endswith("/"))])
    if actual != expected:
        raise ReleaseError(f"archive members disagree; expected={expected}, actual={actual}")
    expected_name = expected_archive_name(metadata["package_version"], metadata["rust_target"])
    if path.name != expected_name:
        raise ReleaseError(f"archive filename {path.name!r} does not match {expected_name!r}")
    binary_name = metadata_name.rsplit("/", 1)[0] + "/" + metadata["binary_filename"]
    binary_bytes = contents[binary_name]
    if sha256_bytes(binary_bytes) != metadata["binary_sha256"]:
        raise ReleaseError("archive binary digest disagrees with RELEASE-METADATA.json")
    if (
        contents[metadata_name.rsplit("/", 1)[0] + "/LICENSE"]
        != expected_release_source_bytes["LICENSE"]
    ):
        raise ReleaseError("archive LICENSE disagrees with repository LICENSE")
    if contents[metadata_name.rsplit("/", 1)[0] + "/THIRD_PARTY_LICENSES.txt"] != (
        expected_release_source_bytes["THIRD_PARTY_LICENSES.txt"]
    ):
        raise ReleaseError("archive third-party license bundle disagrees with repository bundle")
    if contents[metadata_name.rsplit("/", 1)[0] + "/THIRD_PARTY_NOTICES.md"] != (
        expected_release_source_bytes["THIRD_PARTY_NOTICES.md"]
    ):
        raise ReleaseError("archive notices disagree with repository notices")
    return InspectedArchive(
        path,
        metadata,
        metadata_bytes,
        binary_bytes,
        modes[binary_name],
        contents,
        modes,
    )


def verify_archive(
    path: Path,
    *,
    expected_target: str | None = None,
    expected_source_commit: str | None = None,
    expected_source_tree_state: str | None = None,
    expected_tag: str | None = None,
    expect_preflight: bool = False,
    smoke: bool = False,
    root: Path = ROOT,
) -> InspectedArchive:
    inspected = inspect_archive(path, root)
    metadata = inspected.metadata
    if expected_target is not None and metadata["rust_target"] != expected_target:
        raise ReleaseError("package metadata target disagrees with expected target")
    if expected_source_commit is not None and metadata["source_commit"] != expected_source_commit:
        raise ReleaseError("package metadata source commit disagrees with expected commit")
    if (
        expected_source_tree_state is not None
        and metadata["source_tree_state"] != expected_source_tree_state
    ):
        raise ReleaseError("package metadata source tree state disagrees with expected state")
    if expected_tag is not None and metadata["release_tag"] != expected_tag:
        raise ReleaseError("package metadata release tag disagrees with expected tag")
    if expect_preflight and metadata["release_mode"] != "preflight":
        raise ReleaseError("expected a preflight package")
    if smoke:
        smoke_archive(inspected)
    return inspected


def smoke_archive(inspected: InspectedArchive) -> None:
    with tempfile.TemporaryDirectory(prefix="scrub-release-smoke-") as directory_name:
        directory = Path(directory_name)
        package_root = directory / (
            f"scrub-v{inspected.metadata['package_version']}-{inspected.metadata['rust_target']}"
        )
        package_root.mkdir()
        package_root.chmod(0o755)
        for archive_name, data in inspected.members.items():
            relative = PurePosixPath(archive_name).relative_to(package_root.name)
            destination = package_root.joinpath(*relative.parts)
            destination.write_bytes(data)
            destination.chmod(inspected.member_modes[archive_name])
        binary = package_root / inspected.metadata["binary_filename"]
        environment = os.environ.copy()
        environment["NO_COLOR"] = "1"
        help_result = subprocess.run(
            [str(binary), "--help"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=30,
        )
        if help_result.returncode != 0:
            raise ReleaseError("extracted binary failed --help smoke")
        fixture = directory / "release-smoke.txt"
        fixture_bytes = b"scrub release package smoke\n"
        fixture.write_bytes(fixture_bytes)
        inspect_result = subprocess.run(
            [str(binary), "inspect", str(fixture), "--json"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=30,
        )
        if inspect_result.returncode != 0 or inspect_result.stderr:
            raise ReleaseError("extracted binary failed deterministic inspect smoke")
        report = read_json_bytes(inspect_result.stdout, "extracted binary JSON output")
        artifact = report.get("artifact")
        if not isinstance(artifact, dict) or artifact.get("content_sha256") != sha256_bytes(
            fixture_bytes
        ):
            raise ReleaseError("extracted binary inspect smoke reported the wrong fixture digest")


def archive_candidates(input_dir: Path) -> list[Path]:
    return sorted(
        path
        for path in input_dir.iterdir()
        if path.is_file() and (path.suffix == ".zip" or path.name.endswith(".tar.gz"))
    )


def validate_manifest(document: dict[str, Any]) -> None:
    if not isinstance(document, dict):
        raise ReleaseError("release manifest must be an object")
    ensure_exact_keys(document, MANIFEST_KEYS, "release manifest")
    if document["schema_version"] != SCHEMA_VERSION or document["package_name"] != "scrub":
        raise ReleaseError("release manifest schema or package identity is invalid")
    if not isinstance(document["release_mode"], str) or document[
        "release_mode"
    ] not in {"preflight", "tag"}:
        raise ReleaseError("release manifest mode is invalid")
    if document["release_tag"] is not None and not isinstance(document["release_tag"], str):
        raise ReleaseError("release manifest tag must be a string or null")
    if (document["release_mode"] == "tag") != (document["release_tag"] is not None):
        raise ReleaseError("release manifest mode and tag disagree")
    version = validate_version_contract(document["release_tag"])
    if document["package_version"] != version:
        raise ReleaseError("release manifest version disagrees with Cargo manifests")
    validate_commit(document["source_commit"])
    if document["source_tree_state"] not in {"clean_commit", "dirty_worktree"}:
        raise ReleaseError("release manifest source tree state is invalid")
    if document["release_mode"] == "tag" and document["source_tree_state"] != "clean_commit":
        raise ReleaseError("tag release manifest cannot describe a dirty worktree")
    if document["expected_targets"] != list(TARGETS):
        raise ReleaseError("release manifest expected target set is incomplete or reordered")
    rows = document["artifacts"]
    if not isinstance(rows, list) or len(rows) != len(TARGETS):
        raise ReleaseError("release manifest must contain exactly four artifact rows")
    row_targets: list[str] = []
    filenames: list[str] = []
    for row in rows:
        if not isinstance(row, dict):
            raise ReleaseError("release manifest artifact row must be an object")
        ensure_exact_keys(row, MANIFEST_ROW_KEYS, "release manifest artifact")
        target = row["rust_target"]
        if not isinstance(target, str) or target not in TARGET_CONFIG:
            raise ReleaseError(f"release manifest has unknown target: {target!r}")
        row_targets.append(target)
        filenames.append(row["archive_filename"])
        expected_filename = expected_archive_name(version, target)
        if row["archive_filename"] != expected_filename:
            raise ReleaseError("release manifest archive filename disagrees with target/version")
        for field in ("archive_sha256", "binary_sha256", "release_metadata_sha256"):
            if not isinstance(row[field], str) or not SHA256_RE.fullmatch(row[field]):
                raise ReleaseError(f"release manifest has malformed {field}")
        if row["packaging_schema_version"] != SCHEMA_VERSION:
            raise ReleaseError("release manifest has an unknown packaging schema version")
    if row_targets != list(TARGETS) or len(set(row_targets)) != len(TARGETS):
        raise ReleaseError("release manifest targets are missing, duplicate, unknown, or reordered")
    if len(set(filenames)) != len(filenames):
        raise ReleaseError("release manifest has duplicate archive filenames")
    if contains_absolute_path(document):
        raise ReleaseError("release manifest contains an absolute local path")


def build_release_manifest(
    *,
    input_dir: Path,
    source_commit: str,
    source_tree_state: str,
    tag: str | None,
    root: Path = ROOT,
) -> dict[str, Any]:
    validate_commit(source_commit)
    version = validate_version_contract(tag, root)
    packages: dict[str, InspectedArchive] = {}
    candidates = archive_candidates(input_dir)
    for archive in candidates:
        inspected = inspect_archive(archive, root)
        target = inspected.metadata["rust_target"]
        if target in packages:
            raise ReleaseError(f"duplicate release target: {target}")
        packages[target] = inspected
    missing = sorted(set(TARGETS) - set(packages))
    unknown = sorted(set(packages) - set(TARGETS))
    if missing or unknown or len(candidates) != len(TARGETS):
        raise ReleaseError(
            f"release package set is not exact; missing={missing}, unknown={unknown}, "
            f"archive_count={len(candidates)}"
        )
    rows: list[dict[str, Any]] = []
    for target in TARGETS:
        inspected = packages[target]
        metadata = inspected.metadata
        if metadata["source_commit"] != source_commit:
            raise ReleaseError(f"release package source commit disagrees for {target}")
        if metadata["source_tree_state"] != source_tree_state:
            raise ReleaseError(f"release package source tree state disagrees for {target}")
        if metadata["package_version"] != version:
            raise ReleaseError(f"release package version disagrees for {target}")
        if metadata["release_tag"] != tag or metadata["release_mode"] != release_mode(tag):
            raise ReleaseError(f"release package mode/tag disagrees for {target}")
        rows.append(
            {
                "archive_filename": inspected.path.name,
                "archive_sha256": sha256_file(inspected.path),
                "rust_target": target,
                "binary_sha256": metadata["binary_sha256"],
                "release_metadata_sha256": sha256_bytes(inspected.metadata_bytes),
                "packaging_schema_version": metadata["schema_version"],
            }
        )
    document = {
        "schema_version": SCHEMA_VERSION,
        "release_mode": release_mode(tag),
        "package_name": "scrub",
        "package_version": version,
        "release_tag": tag,
        "source_commit": source_commit,
        "source_tree_state": source_tree_state,
        "expected_targets": list(TARGETS),
        "artifacts": rows,
    }
    validate_manifest(document)
    return document


def checksum_bytes(files: Iterable[Path]) -> bytes:
    paths = sorted(files, key=lambda path: path.name)
    names = [path.name for path in paths]
    if len(names) != len(set(names)) or "SHA256SUMS" in names:
        raise ReleaseError("checksum inputs have duplicate names or include SHA256SUMS")
    return "".join(f"{sha256_file(path)}  {path.name}\n" for path in paths).encode("ascii")


def write_atomic(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(value)
    temporary.replace(path)


def assemble_release(
    *,
    input_dir: Path,
    output_dir: Path,
    source_commit: str,
    source_tree_state: str,
    tag: str | None,
    root: Path = ROOT,
) -> tuple[Path, Path]:
    document = build_release_manifest(
        input_dir=input_dir,
        source_commit=source_commit,
        source_tree_state=source_tree_state,
        tag=tag,
        root=root,
    )
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "release-manifest.json"
    checksums_path = output_dir / "SHA256SUMS"
    write_atomic(manifest_path, json_bytes(document))
    archives = [input_dir / row["archive_filename"] for row in document["artifacts"]]
    write_atomic(checksums_path, checksum_bytes([*archives, manifest_path]))
    verify_release_output(output_dir, archive_dir=input_dir, root=root)
    return manifest_path, checksums_path


def parse_checksums(value: bytes) -> dict[str, str]:
    try:
        text = value.decode("ascii")
    except UnicodeDecodeError as error:
        raise ReleaseError("SHA256SUMS must be ASCII") from error
    if "\r" in text or (text and not text.endswith("\n")):
        raise ReleaseError("SHA256SUMS must use LF lines with a final newline")
    result: dict[str, str] = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([^/\\]+)", line)
        if not match:
            raise ReleaseError(f"malformed SHA256SUMS line: {line!r}")
        digest, filename = match.groups()
        if filename in result or filename == "SHA256SUMS":
            raise ReleaseError("SHA256SUMS has a duplicate name or includes itself")
        result[filename] = digest
    if list(result) != sorted(result):
        raise ReleaseError("SHA256SUMS filenames are not sorted")
    return result


def verify_release_output(
    release_dir: Path, *, archive_dir: Path | None = None, root: Path = ROOT
) -> dict[str, Any]:
    archive_dir = release_dir if archive_dir is None else archive_dir
    manifest_path = release_dir / "release-manifest.json"
    checksums_path = release_dir / "SHA256SUMS"
    document = read_json_bytes(manifest_path.read_bytes(), "release-manifest.json")
    validate_manifest(document)
    expected_names = sorted(
        [*(row["archive_filename"] for row in document["artifacts"]), manifest_path.name]
    )
    checksums = parse_checksums(checksums_path.read_bytes())
    if list(checksums) != expected_names:
        raise ReleaseError("SHA256SUMS does not cover the exact release asset set")

    expected_archive_names = {row["archive_filename"] for row in document["artifacts"]}
    if archive_dir.resolve() == release_dir.resolve():
        expected_directory_names = expected_archive_names | {
            manifest_path.name,
            checksums_path.name,
        }
        actual_directory_names = {path.name for path in release_dir.iterdir()}
        if actual_directory_names != expected_directory_names or any(
            not path.is_file() for path in release_dir.iterdir()
        ):
            raise ReleaseError("release directory does not contain the exact six release assets")
    else:
        actual_release_names = {path.name for path in release_dir.iterdir()}
        actual_archive_names = {path.name for path in archive_dir.iterdir()}
        if actual_release_names != {manifest_path.name, checksums_path.name} or any(
            not path.is_file() for path in release_dir.iterdir()
        ):
            raise ReleaseError("release metadata directory contains unexpected entries")
        if actual_archive_names != expected_archive_names or any(
            not path.is_file() for path in archive_dir.iterdir()
        ):
            raise ReleaseError("release archive directory is not the exact four-target set")
    if checksums[manifest_path.name] != sha256_file(manifest_path):
        raise ReleaseError("SHA256SUMS release-manifest digest drifted")
    for row in document["artifacts"]:
        archive = archive_dir / row["archive_filename"]
        if not archive.is_file():
            raise ReleaseError(f"release archive is missing: {archive.name}")
        if sha256_file(archive) != row["archive_sha256"]:
            raise ReleaseError(f"release manifest archive digest disagrees: {archive.name}")
        if checksums[archive.name] != row["archive_sha256"]:
            raise ReleaseError(f"SHA256SUMS archive digest disagrees: {archive.name}")
        inspected = inspect_archive(archive, root)
        if inspected.metadata["rust_target"] != row["rust_target"]:
            raise ReleaseError("release manifest target disagrees with package metadata")
        if inspected.metadata["binary_sha256"] != row["binary_sha256"]:
            raise ReleaseError("release manifest binary digest disagrees with package metadata")
        if sha256_bytes(inspected.metadata_bytes) != row["release_metadata_sha256"]:
            raise ReleaseError("release manifest metadata digest disagrees with package bytes")
        if inspected.metadata["source_commit"] != document["source_commit"]:
            raise ReleaseError("release manifest source commit disagrees with package metadata")
        if inspected.metadata["source_tree_state"] != document["source_tree_state"]:
            raise ReleaseError("release manifest source tree state disagrees with package metadata")
        if inspected.metadata["package_version"] != document["package_version"]:
            raise ReleaseError("release manifest version disagrees with package metadata")
        if inspected.metadata["release_tag"] != document["release_tag"]:
            raise ReleaseError("release manifest tag disagrees with package metadata")
    return document


def add_mode_arguments(parser: argparse.ArgumentParser) -> None:
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--tag")
    group.add_argument("--preflight", action="store_true")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    version = subparsers.add_parser("validate-version")
    add_mode_arguments(version)

    package = subparsers.add_parser("package")
    package.add_argument("--binary", type=Path, required=True)
    package.add_argument("--target", choices=TARGETS, required=True)
    package.add_argument("--source-commit", required=True)
    package.add_argument(
        "--source-tree-state", choices=("clean_commit", "dirty_worktree"), required=True
    )
    package.add_argument("--rustc-version", required=True)
    package.add_argument("--cargo-version", required=True)
    package.add_argument("--output-dir", type=Path, required=True)
    add_mode_arguments(package)

    verify = subparsers.add_parser("verify-package")
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--expected-target", choices=TARGETS)
    verify.add_argument("--source-commit")
    verify.add_argument("--source-tree-state", choices=("clean_commit", "dirty_worktree"))
    verify.add_argument("--smoke", action="store_true")
    add_mode_arguments(verify)

    assemble = subparsers.add_parser("assemble")
    assemble.add_argument("--input-dir", type=Path, required=True)
    assemble.add_argument("--output-dir", type=Path, required=True)
    assemble.add_argument("--source-commit", required=True)
    assemble.add_argument(
        "--source-tree-state", choices=("clean_commit", "dirty_worktree"), required=True
    )
    add_mode_arguments(assemble)

    verify_release = subparsers.add_parser("verify-release")
    verify_release.add_argument("--release-dir", type=Path, required=True)
    verify_release.add_argument("--archive-dir", type=Path)
    return parser


def selected_tag(arguments: argparse.Namespace) -> str | None:
    return getattr(arguments, "tag", None)


def main(argv: list[str] | None = None) -> int:
    arguments = build_parser().parse_args(argv)
    try:
        validate_third_party_license_bundle()
        if arguments.command == "validate-version":
            version = validate_version_contract(selected_tag(arguments))
            print(version)
        elif arguments.command == "package":
            output = package_archive(
                binary=arguments.binary,
                target=arguments.target,
                source_commit=arguments.source_commit,
                source_tree_state=arguments.source_tree_state,
                rustc_version=arguments.rustc_version,
                cargo_version=arguments.cargo_version,
                tag=selected_tag(arguments),
                output_dir=arguments.output_dir,
            )
            print(output.as_posix())
        elif arguments.command == "verify-package":
            inspected = verify_archive(
                arguments.archive,
                expected_target=arguments.expected_target,
                expected_source_commit=arguments.source_commit,
                expected_source_tree_state=arguments.source_tree_state,
                expected_tag=selected_tag(arguments),
                expect_preflight=arguments.preflight,
                smoke=arguments.smoke,
            )
            print(
                json.dumps(
                    {
                        "archive": arguments.archive.as_posix(),
                        "archive_sha256": sha256_file(arguments.archive),
                        "binary_sha256": inspected.metadata["binary_sha256"],
                        "rust_target": inspected.metadata["rust_target"],
                        "verified": True,
                    },
                    sort_keys=True,
                    separators=(",", ":"),
                )
            )
        elif arguments.command == "assemble":
            manifest, checksums = assemble_release(
                input_dir=arguments.input_dir,
                output_dir=arguments.output_dir,
                source_commit=arguments.source_commit,
                source_tree_state=arguments.source_tree_state,
                tag=selected_tag(arguments),
            )
            print(
                json.dumps(
                    {
                        "release_manifest": manifest.as_posix(),
                        "sha256sums": checksums.as_posix(),
                    },
                    sort_keys=True,
                    separators=(",", ":"),
                )
            )
        else:
            document = verify_release_output(
                arguments.release_dir, archive_dir=arguments.archive_dir
            )
            print(
                json.dumps(
                    {
                        "release_tag": document["release_tag"],
                        "source_commit": document["source_commit"],
                        "verified": True,
                    },
                    sort_keys=True,
                    separators=(",", ":"),
                )
            )
    except (OSError, ReleaseError, subprocess.SubprocessError) as error:
        print(f"release contract failed: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
