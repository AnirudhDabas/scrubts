#!/usr/bin/env python3
"""Generate and verify scrub's exact third-party license-text bundle."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tomllib
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_PATH = "THIRD_PARTY_LICENSES.txt"
MANIFEST_PATH = "third_party/license-manifest.json"
FALLBACK_MANIFEST_PATH = "third_party/license-fallbacks/manifest.json"
FALLBACK_TEXT_DIRECTORY = "third_party/license-fallbacks/texts"
FORMAT_VERSION = 1
EXPECTED_PACKAGE_COUNT = 251
TARGETS = (
    ("x86_64-unknown-linux-gnu", "linux-x86_64"),
    ("aarch64-apple-darwin", "macos-aarch64"),
    ("x86_64-apple-darwin", "macos-x86_64"),
    ("x86_64-pc-windows-msvc", "windows-x86_64"),
)
DIRECT_PACKAGES = {
    ("c2pa", "0.90.12"),
    ("sha2", "0.11.0"),
    ("unicode-normalization", "0.1.25"),
}
SOURCE_FILE_RE = re.compile(
    r"^(?:licen[cs]e(?:[-._].*)?|copying(?:[-._].*)?|copyright(?:[-._].*)?|"
    r"notice(?:[-._].*)?|unlicense|authors)$",
    re.IGNORECASE,
)
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


class LicenseBundleError(ValueError):
    """The third-party license bundle contract was not satisfied."""


@dataclass(frozen=True)
class InventoryPackage:
    name: str
    version: str
    relationship: str
    declared_license: str
    targets: tuple[str, ...]
    source: str
    checksum: str


@dataclass(frozen=True)
class LicenseFile:
    filename: str
    origin: str
    value: bytes

    @property
    def digest(self) -> str:
        return hashlib.sha256(self.value).hexdigest()


@dataclass(frozen=True)
class BundledPackage:
    package: InventoryPackage
    files: tuple[LicenseFile, ...]
    file_status: str


def exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        raise LicenseBundleError(
            f"{label} fields disagree; missing={sorted(expected - actual)}, "
            f"unknown={sorted(actual - expected)}"
        )


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_lock(root: Path = ROOT) -> dict[tuple[str, str], tuple[str, str]]:
    try:
        with (root / "Cargo.lock").open("rb") as handle:
            document = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise LicenseBundleError(f"cannot read Cargo.lock: {error}") from error
    records: dict[tuple[str, str], tuple[str, str]] = {}
    for package in document.get("package", []):
        if not isinstance(package, dict):
            raise LicenseBundleError("Cargo.lock package entry is not a table")
        name = package.get("name")
        version = package.get("version")
        source = package.get("source")
        checksum = package.get("checksum")
        if not all(isinstance(item, str) for item in (name, version)):
            raise LicenseBundleError("Cargo.lock package identity is malformed")
        if source is None and checksum is None:
            continue
        if not isinstance(source, str) or not isinstance(checksum, str):
            raise LicenseBundleError(f"locked external package has incomplete source: {name} {version}")
        if not SHA256_RE.fullmatch(checksum):
            raise LicenseBundleError(f"locked package checksum is malformed: {name} {version}")
        key = (name, version)
        if key in records:
            raise LicenseBundleError(f"ambiguous locked package identity: {name} {version}")
        records[key] = (source, checksum)
    return records


def parse_inventory(root: Path = ROOT) -> dict[tuple[str, str], InventoryPackage]:
    try:
        lines = (root / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as error:
        raise LicenseBundleError(f"cannot read THIRD_PARTY_NOTICES.md: {error}") from error
    pattern = re.compile(
        r"^\| \[([^]]+)\]\(([^)]+)\) \| ([^|]+) \| ([^|]+) \| `([^`]*)` \| ([^|]+) \|$"
    )
    lock = load_lock(root)
    records: dict[tuple[str, str], InventoryPackage] = {}
    for line in lines:
        match = pattern.fullmatch(line)
        if match is None:
            continue
        name, url, version, relationship, declared_license, target_text = (
            item.strip() for item in match.groups()
        )
        key = (name, version)
        if key in records:
            raise LicenseBundleError(f"duplicate notice inventory package: {name} {version}")
        if url != f"https://crates.io/crates/{name}/{version}":
            raise LicenseBundleError(f"notice inventory URL disagrees: {name} {version}")
        if key not in lock:
            raise LicenseBundleError(f"notice inventory package is not locked: {name} {version}")
        if target_text == "all four":
            targets = tuple(label for _target, label in TARGETS)
        else:
            targets = tuple(item.strip() for item in target_text.split(","))
        valid_targets = tuple(label for _target, label in TARGETS)
        if not targets or any(target not in valid_targets for target in targets):
            raise LicenseBundleError(f"notice inventory targets are malformed: {name} {version}")
        expected_relationship = "direct" if key in DIRECT_PACKAGES else "transitive"
        if relationship != expected_relationship:
            raise LicenseBundleError(f"notice relationship disagrees: {name} {version}")
        source, checksum = lock[key]
        records[key] = InventoryPackage(
            name,
            version,
            relationship,
            declared_license,
            targets,
            source,
            checksum,
        )
    if len(records) != EXPECTED_PACKAGE_COUNT:
        raise LicenseBundleError(
            f"notice inventory package count disagrees: {len(records)} != {EXPECTED_PACKAGE_COUNT}"
        )
    return records


def cargo_tree_packages(root: Path = ROOT) -> dict[tuple[str, str], tuple[str, set[str]]]:
    lock = load_lock(root)
    union: dict[tuple[str, str], tuple[str, set[str]]] = {}
    display = re.compile(r"^(.+) v([^ ]+)(?: \(.*\))?$")
    for target, target_label in TARGETS:
        try:
            completed = subprocess.run(
                [
                    "cargo",
                    "tree",
                    "-p",
                    "scrub",
                    "--locked",
                    "--offline",
                    "--edges",
                    "normal,build",
                    "--target",
                    target,
                    "--prefix",
                    "none",
                    "--format",
                    "{p}|{l}",
                ],
                cwd=root,
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
            )
        except (OSError, subprocess.CalledProcessError) as error:
            detail = getattr(error, "stderr", "")
            raise LicenseBundleError(f"cannot reconstruct cargo tree for {target}: {detail}") from error
        for raw_line in completed.stdout.splitlines():
            line = raw_line.removesuffix(" (*)")
            try:
                package_text, declared_license = line.rsplit("|", 1)
            except ValueError as error:
                raise LicenseBundleError(f"unexpected cargo tree row: {raw_line!r}") from error
            match = display.fullmatch(package_text)
            if match is None:
                raise LicenseBundleError(f"unexpected cargo package display: {package_text!r}")
            key = (match.group(1), match.group(2))
            if key not in lock:
                continue
            if key in union and union[key][0] != declared_license:
                raise LicenseBundleError(f"cargo license metadata changed across targets: {key}")
            if key not in union:
                union[key] = (declared_license, set())
            union[key][1].add(target_label)
    if len(union) != EXPECTED_PACKAGE_COUNT:
        raise LicenseBundleError(
            f"four-target production union count disagrees: {len(union)} != {EXPECTED_PACKAGE_COUNT}"
        )
    return union


def verify_inventory_against_cargo(root: Path = ROOT) -> dict[tuple[str, str], InventoryPackage]:
    inventory = parse_inventory(root)
    cargo_union = cargo_tree_packages(root)
    if set(inventory) != set(cargo_union):
        raise LicenseBundleError(
            f"notice inventory and cargo union disagree; "
            f"missing={sorted(set(cargo_union) - set(inventory))}, "
            f"extra={sorted(set(inventory) - set(cargo_union))}"
        )
    target_order = tuple(label for _target, label in TARGETS)
    for key, package in inventory.items():
        declared_license, targets = cargo_union[key]
        ordered_targets = tuple(label for label in target_order if label in targets)
        if declared_license != package.declared_license:
            raise LicenseBundleError(f"declared license metadata disagrees: {key}")
        if ordered_targets != package.targets:
            raise LicenseBundleError(f"target membership disagrees: {key}")
    return inventory


def cargo_cache_root() -> Path:
    configured = os.environ.get("CARGO_HOME")
    return Path(configured) if configured else Path.home() / ".cargo"


def crate_archive(package: InventoryPackage, cargo_home: Path | None = None) -> Path:
    home = cargo_cache_root() if cargo_home is None else cargo_home
    cache = home / "registry" / "cache"
    candidates = sorted(cache.glob(f"*/{package.name}-{package.version}.crate"))
    matching = [path for path in candidates if sha256_file(path) == package.checksum]
    if len(matching) != 1:
        raise LicenseBundleError(
            f"exact crate archive unavailable or ambiguous: {package.name} {package.version} "
            f"checksum={package.checksum}"
        )
    return matching[0]


def read_crate_source(
    package: InventoryPackage, cargo_home: Path | None = None
) -> tuple[tuple[LicenseFile, ...], dict[str, Any]]:
    archive_path = crate_archive(package, cargo_home)
    root_name = f"{package.name}-{package.version}"
    files: list[LicenseFile] = []
    vcs_info: dict[str, Any] | None = None
    seen_names: set[str] = set()
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            for member in archive:
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or not path.parts:
                    raise LicenseBundleError(f"unsafe member in exact crate archive: {member.name}")
                if path.parts[0] != root_name:
                    raise LicenseBundleError(f"crate archive root disagrees: {member.name}")
                if len(path.parts) < 2 or not member.isfile():
                    continue
                relative = PurePosixPath(*path.parts[1:])
                filename = relative.as_posix()
                if filename == ".cargo_vcs_info.json":
                    extracted = archive.extractfile(member)
                    if extracted is None:
                        raise LicenseBundleError(f"cannot read VCS identity for {package.name}")
                    try:
                        parsed = json.loads(extracted.read().decode("utf-8"))
                    except (UnicodeError, json.JSONDecodeError) as error:
                        raise LicenseBundleError(f"invalid VCS identity for {package.name}") from error
                    if not isinstance(parsed, dict):
                        raise LicenseBundleError(f"invalid VCS identity for {package.name}")
                    vcs_info = parsed
                if not SOURCE_FILE_RE.fullmatch(relative.name):
                    continue
                folded = filename.casefold()
                if folded in seen_names:
                    raise LicenseBundleError(f"case-colliding license files in {package.name}")
                seen_names.add(folded)
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise LicenseBundleError(f"cannot read license file from {package.name}: {filename}")
                value = extracted.read()
                try:
                    value.decode("utf-8")
                except UnicodeDecodeError as error:
                    raise LicenseBundleError(
                        f"license file is not UTF-8 and cannot enter text bundle: {package.name} {filename}"
                    ) from error
                files.append(LicenseFile(filename, f"crate-package:{filename}", value))
    except (OSError, tarfile.TarError) as error:
        raise LicenseBundleError(f"cannot inspect exact crate archive {archive_path}: {error}") from error
    files.sort(key=lambda item: (item.filename.casefold(), item.filename))
    return tuple(files), vcs_info or {}


def load_fallbacks(root: Path = ROOT) -> dict[tuple[str, str], dict[str, Any]]:
    path = root / FALLBACK_MANIFEST_PATH
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise LicenseBundleError(f"cannot read fallback manifest: {error}") from error
    if not isinstance(document, dict):
        raise LicenseBundleError("fallback manifest must be an object")
    exact_keys(document, {"schema_version", "candidate_file_pattern", "packages"}, "fallback manifest")
    if document["schema_version"] != FORMAT_VERSION:
        raise LicenseBundleError("fallback manifest schema version disagrees")
    if document["candidate_file_pattern"] != SOURCE_FILE_RE.pattern:
        raise LicenseBundleError("fallback candidate-file contract disagrees")
    packages = document["packages"]
    if not isinstance(packages, list):
        raise LicenseBundleError("fallback packages must be an array")
    records: dict[tuple[str, str], dict[str, Any]] = {}
    package_keys = {
        "package",
        "version",
        "repository",
        "revision",
        "path_in_vcs",
        "candidate_files",
    }
    file_keys = {"path", "git_blob_sha1", "byte_length", "sha256"}
    for entry in packages:
        if not isinstance(entry, dict):
            raise LicenseBundleError("fallback package entry must be an object")
        exact_keys(entry, package_keys, "fallback package")
        key = (entry["package"], entry["version"])
        if not all(isinstance(item, str) for item in key):
            raise LicenseBundleError("fallback package identity is malformed")
        if key in records:
            raise LicenseBundleError(f"duplicate fallback package: {key}")
        if not isinstance(entry["repository"], str) or not entry["repository"].startswith(
            "https://github.com/"
        ):
            raise LicenseBundleError(f"fallback repository is not canonical: {key}")
        if not isinstance(entry["revision"], str) or not COMMIT_RE.fullmatch(entry["revision"]):
            raise LicenseBundleError(f"fallback revision is malformed: {key}")
        if not isinstance(entry["path_in_vcs"], str):
            raise LicenseBundleError(f"fallback path_in_vcs is malformed: {key}")
        candidate_files = entry["candidate_files"]
        if not isinstance(candidate_files, list):
            raise LicenseBundleError(f"fallback candidate_files is malformed: {key}")
        names: list[str] = []
        for source_file in candidate_files:
            if not isinstance(source_file, dict):
                raise LicenseBundleError(f"fallback file entry is malformed: {key}")
            exact_keys(source_file, file_keys, "fallback source file")
            filename = source_file["path"]
            if (
                not isinstance(filename, str)
                or PurePosixPath(filename).is_absolute()
                or ".." in PurePosixPath(filename).parts
                or not SOURCE_FILE_RE.fullmatch(PurePosixPath(filename).name)
            ):
                raise LicenseBundleError(f"fallback filename is not canonical: {key}")
            if filename.casefold() in (item.casefold() for item in names):
                raise LicenseBundleError(f"fallback filenames collide: {key}")
            names.append(filename)
            digest = source_file["sha256"]
            if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
                raise LicenseBundleError(f"fallback SHA-256 is malformed: {key}")
            if not isinstance(source_file["git_blob_sha1"], str) or not COMMIT_RE.fullmatch(
                source_file["git_blob_sha1"]
            ):
                raise LicenseBundleError(f"fallback Git blob identity is malformed: {key}")
            if not isinstance(source_file["byte_length"], int) or source_file["byte_length"] < 0:
                raise LicenseBundleError(f"fallback byte length is malformed: {key}")
            text_path = root / FALLBACK_TEXT_DIRECTORY / f"{digest}.txt"
            try:
                value = text_path.read_bytes()
                value.decode("utf-8")
            except (OSError, UnicodeError) as error:
                raise LicenseBundleError(f"cannot read fallback text {digest}: {error}") from error
            if len(value) != source_file["byte_length"] or hashlib.sha256(value).hexdigest() != digest:
                raise LicenseBundleError(f"fallback text bytes disagree: {key} {filename}")
        if names != sorted(names, key=lambda item: (item.casefold(), item)):
            raise LicenseBundleError(f"fallback candidate files are reordered: {key}")
        records[key] = entry
    return records


def vcs_identity(value: dict[str, Any]) -> tuple[str, str]:
    try:
        revision = value["git"]["sha1"]
        path_in_vcs = value["path_in_vcs"]
    except (KeyError, TypeError) as error:
        raise LicenseBundleError("crate VCS identity is incomplete") from error
    if not isinstance(revision, str) or not COMMIT_RE.fullmatch(revision):
        raise LicenseBundleError("crate VCS revision is malformed")
    if not isinstance(path_in_vcs, str):
        raise LicenseBundleError("crate VCS path is malformed")
    return revision, path_in_vcs


def build_packages_from_sources(
    root: Path = ROOT, cargo_home: Path | None = None
) -> tuple[BundledPackage, ...]:
    inventory = verify_inventory_against_cargo(root)
    fallbacks = load_fallbacks(root)
    packages: list[BundledPackage] = []
    used_fallbacks: set[tuple[str, str]] = set()
    for key in sorted(inventory):
        package = inventory[key]
        files, vcs = read_crate_source(package, cargo_home)
        if files:
            if key in fallbacks:
                raise LicenseBundleError(f"unexpected fallback for packaged license files: {key}")
            packages.append(BundledPackage(package, files, "preserved-from-exact-crate-package"))
            continue
        if key not in fallbacks:
            raise LicenseBundleError(
                f"no authentic license source resolved: {package.name} {package.version} "
                f"declared={package.declared_license!r} source={package.source!r}"
            )
        fallback = fallbacks[key]
        revision, path_in_vcs = vcs_identity(vcs)
        if revision != fallback["revision"] or path_in_vcs != fallback["path_in_vcs"]:
            raise LicenseBundleError(f"fallback VCS identity disagrees with exact crate: {key}")
        fallback_files: list[LicenseFile] = []
        for source_file in fallback["candidate_files"]:
            digest = source_file["sha256"]
            value = (root / FALLBACK_TEXT_DIRECTORY / f"{digest}.txt").read_bytes()
            origin = (
                f"upstream:{fallback['repository']}@{fallback['revision']}/{source_file['path']}"
            )
            fallback_files.append(LicenseFile(source_file["path"], origin, value))
        status = (
            "preserved-from-pinned-upstream"
            if fallback_files
            else "no-candidate-file-in-exact-crate-or-pinned-upstream"
        )
        packages.append(BundledPackage(package, tuple(fallback_files), status))
        used_fallbacks.add(key)
    if used_fallbacks != set(fallbacks):
        raise LicenseBundleError(
            f"fallback package set disagrees; unused={sorted(set(fallbacks) - used_fallbacks)}"
        )
    return tuple(packages)


def json_line(value: dict[str, Any]) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(
        "utf-8"
    )


def json_bytes(value: dict[str, Any]) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")


def manifest_document(packages: tuple[BundledPackage, ...]) -> dict[str, Any]:
    mappings = sum(len(item.files) for item in packages)
    unique_texts = {source_file.digest for item in packages for source_file in item.files}
    return {
        "mapping_count": mappings,
        "package_count": len(packages),
        "packages": [
            {
                "checksum": bundled.package.checksum,
                "file_status": bundled.file_status,
                "files": [
                    {
                        "byte_length": len(source_file.value),
                        "filename": source_file.filename,
                        "origin": source_file.origin,
                        "sha256": source_file.digest,
                    }
                    for source_file in bundled.files
                ],
                "name": bundled.package.name,
                "source": bundled.package.source,
                "version": bundled.package.version,
            }
            for bundled in packages
        ],
        "schema_version": FORMAT_VERSION,
        "unique_text_count": len(unique_texts),
    }


def render_manifest(packages: tuple[BundledPackage, ...]) -> bytes:
    return json_bytes(manifest_document(packages))


def render_bundle(packages: tuple[BundledPackage, ...]) -> bytes:
    texts: dict[str, bytes] = {}
    mapping_count = sum(len(package.files) for package in packages)
    output = bytearray(
        (
            "scrub.ts third-party license texts\n"
            f"format-version: {FORMAT_VERSION}\n"
            f"package-count: {len(packages)}\n"
            f"file-mapping-count: {mapping_count}\n"
            "source-contract: locked four-target normal/build production dependency union\n"
            "content-contract: exact UTF-8 bytes from canonical crate files or pinned upstream fallback files\n"
        ).encode("utf-8")
    )
    for bundled in packages:
        package = bundled.package
        output.extend(b"@@PACKAGE ")
        output.extend(
            json_line(
                {
                    "checksum": package.checksum,
                    "declared_license": package.declared_license,
                    "file_count": len(bundled.files),
                    "file_status": bundled.file_status,
                    "name": package.name,
                    "relationship": package.relationship,
                    "source": package.source,
                    "targets": list(package.targets),
                    "version": package.version,
                }
            )
        )
        output.extend(b"\n")
        for source_file in bundled.files:
            digest = source_file.digest
            texts.setdefault(digest, source_file.value)
            if texts[digest] != source_file.value:
                raise LicenseBundleError("SHA-256 collision in license texts")
            output.extend(b"@@FILE ")
            output.extend(
                json_line(
                    {
                        "byte_length": len(source_file.value),
                        "filename": source_file.filename,
                        "origin": source_file.origin,
                        "sha256": digest,
                        "text_id": f"sha256:{digest}",
                    }
                )
            )
            output.extend(b"\n")
    output.extend(b"@@END-MAP\n")
    for digest in sorted(texts):
        value = texts[digest]
        output.extend(b"@@TEXT ")
        output.extend(json_line({"byte_length": len(value), "sha256": digest}))
        output.extend(b"\n")
        output.extend(value)
        output.extend(b"\n@@END-TEXT\n")
    return bytes(output)


def parse_json_record(value: bytes, label: str) -> dict[str, Any]:
    try:
        parsed = json.loads(value.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise LicenseBundleError(f"invalid {label} JSON: {error}") from error
    if not isinstance(parsed, dict):
        raise LicenseBundleError(f"{label} must be an object")
    if json_line(parsed) != value:
        raise LicenseBundleError(f"{label} JSON is not canonical")
    return parsed


def parse_bundle(value: bytes) -> tuple[list[dict[str, Any]], dict[str, bytes]]:
    prefix = b"scrub.ts third-party license texts\n"
    if not value.startswith(prefix):
        raise LicenseBundleError("license bundle header is missing")
    marker = b"@@END-MAP\n"
    marker_at = value.find(marker)
    if marker_at < 0:
        raise LicenseBundleError("license bundle map terminator is missing")
    map_bytes = value[:marker_at].splitlines()
    if len(map_bytes) < 6:
        raise LicenseBundleError("license bundle headers are incomplete")
    expected_headers = [
        b"scrub.ts third-party license texts",
        f"format-version: {FORMAT_VERSION}".encode("ascii"),
        f"package-count: {EXPECTED_PACKAGE_COUNT}".encode("ascii"),
    ]
    if map_bytes[:3] != expected_headers:
        raise LicenseBundleError("license bundle fixed headers disagree")
    mapping_header = map_bytes[3]
    mapping_prefix = b"file-mapping-count: "
    if not mapping_header.startswith(mapping_prefix) or not mapping_header[
        len(mapping_prefix) :
    ].isdigit():
        raise LicenseBundleError("license bundle mapping count is malformed")
    if map_bytes[4:6] != [
        b"source-contract: locked four-target normal/build production dependency union",
        b"content-contract: exact UTF-8 bytes from canonical crate files or pinned upstream fallback files",
    ]:
        raise LicenseBundleError("license bundle source/content contract disagrees")
    packages: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    file_count = 0
    for line in map_bytes[6:]:
        if line.startswith(b"@@PACKAGE "):
            if current is not None and len(current["files"]) != current["package"]["file_count"]:
                raise LicenseBundleError("license bundle package file count disagrees")
            package = parse_json_record(line[len(b"@@PACKAGE ") :], "package record")
            current = {"package": package, "files": []}
            packages.append(current)
        elif line.startswith(b"@@FILE "):
            if current is None:
                raise LicenseBundleError("license bundle file precedes a package")
            source_file = parse_json_record(line[len(b"@@FILE ") :], "file record")
            current["files"].append(source_file)
            file_count += 1
        else:
            raise LicenseBundleError("license bundle mapping record is malformed")
    if current is not None and len(current["files"]) != current["package"]["file_count"]:
        raise LicenseBundleError("license bundle package file count disagrees")
    if mapping_header != f"file-mapping-count: {file_count}".encode("ascii"):
        raise LicenseBundleError("license bundle mapping count disagrees")
    if len(packages) != EXPECTED_PACKAGE_COUNT:
        raise LicenseBundleError("license bundle package records are incomplete")
    position = marker_at + len(marker)
    texts: dict[str, bytes] = {}
    text_order: list[str] = []
    while position < len(value):
        line_end = value.find(b"\n", position)
        if line_end < 0 or not value[position:line_end].startswith(b"@@TEXT "):
            raise LicenseBundleError("license bundle text header is malformed")
        metadata = parse_json_record(value[position + len(b"@@TEXT ") : line_end], "text record")
        exact_keys(metadata, {"byte_length", "sha256"}, "text record")
        digest = metadata["sha256"]
        length = metadata["byte_length"]
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise LicenseBundleError("license text digest is malformed")
        if not isinstance(length, int) or length < 0:
            raise LicenseBundleError("license text byte length is malformed")
        content_at = line_end + 1
        content_end = content_at + length
        if content_end > len(value):
            raise LicenseBundleError("license text is truncated")
        content = value[content_at:content_end]
        if value[content_end : content_end + len(b"\n@@END-TEXT\n")] != b"\n@@END-TEXT\n":
            raise LicenseBundleError("license text terminator is malformed")
        if hashlib.sha256(content).hexdigest() != digest:
            raise LicenseBundleError("license text digest disagrees")
        if digest in texts:
            raise LicenseBundleError("duplicate canonical license text identity")
        try:
            content.decode("utf-8")
        except UnicodeDecodeError as error:
            raise LicenseBundleError("bundled license text is not UTF-8") from error
        texts[digest] = content
        text_order.append(digest)
        position = content_end + len(b"\n@@END-TEXT\n")
    if text_order != sorted(text_order):
        raise LicenseBundleError("canonical license texts are reordered")
    for bundled in packages:
        for source_file in bundled["files"]:
            exact_keys(
                source_file,
                {"byte_length", "filename", "origin", "sha256", "text_id"},
                "file record",
            )
            digest = source_file["sha256"]
            if source_file["text_id"] != f"sha256:{digest}" or digest not in texts:
                raise LicenseBundleError("license file text identity is unresolved")
            if source_file["byte_length"] != len(texts[digest]):
                raise LicenseBundleError("license file byte length disagrees")
    return packages, texts


def parse_canonical_json(value: bytes, label: str) -> dict[str, Any]:
    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        parsed: dict[str, Any] = {}
        for key, item in pairs:
            if key in parsed:
                raise LicenseBundleError(f"duplicate object key in {label}: {key!r}")
            parsed[key] = item
        return parsed

    try:
        document = json.loads(
            value.decode("utf-8"), object_pairs_hook=reject_duplicate_keys
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise LicenseBundleError(f"cannot parse {label}: {error}") from error
    if not isinstance(document, dict):
        raise LicenseBundleError(f"{label} must be an object")
    if json_bytes(document) != value:
        raise LicenseBundleError(f"{label} is not canonical JSON")
    return document


def validate_manifest(
    document: dict[str, Any],
    inventory: dict[tuple[str, str], InventoryPackage],
    fallbacks: dict[tuple[str, str], dict[str, Any]],
) -> dict[tuple[str, str], dict[str, Any]]:
    exact_keys(
        document,
        {
            "mapping_count",
            "package_count",
            "packages",
            "schema_version",
            "unique_text_count",
        },
        "license manifest",
    )
    if document["schema_version"] != FORMAT_VERSION:
        raise LicenseBundleError("license manifest schema version disagrees")
    if document["package_count"] != EXPECTED_PACKAGE_COUNT:
        raise LicenseBundleError("license manifest package count disagrees")
    packages = document["packages"]
    if not isinstance(packages, list):
        raise LicenseBundleError("license manifest packages must be an array")
    records: dict[tuple[str, str], dict[str, Any]] = {}
    actual_order: list[tuple[str, str]] = []
    mapping_count = 0
    text_digests: set[str] = set()
    fallback_seen: set[tuple[str, str]] = set()
    for entry in packages:
        if not isinstance(entry, dict):
            raise LicenseBundleError("license manifest package must be an object")
        exact_keys(
            entry,
            {"checksum", "file_status", "files", "name", "source", "version"},
            "license manifest package",
        )
        if not all(isinstance(entry[field], str) for field in ("name", "version")):
            raise LicenseBundleError("license manifest package identity is malformed")
        key = (entry["name"], entry["version"])
        if key in records:
            raise LicenseBundleError(f"duplicate license manifest package: {key}")
        if key not in inventory:
            raise LicenseBundleError(f"license manifest has an extra package: {key}")
        expected_package = inventory[key]
        if (
            entry["source"] != expected_package.source
            or entry["checksum"] != expected_package.checksum
            or not SHA256_RE.fullmatch(entry["checksum"])
        ):
            raise LicenseBundleError(f"license manifest source identity disagrees: {key}")
        files = entry["files"]
        if not isinstance(files, list):
            raise LicenseBundleError(f"license manifest files are malformed: {key}")
        filenames: list[str] = []
        folded_names: set[str] = set()
        for source_file in files:
            if not isinstance(source_file, dict):
                raise LicenseBundleError(f"license manifest file is malformed: {key}")
            exact_keys(
                source_file,
                {"byte_length", "filename", "origin", "sha256"},
                "license manifest file",
            )
            filename = source_file["filename"]
            if (
                not isinstance(filename, str)
                or PurePosixPath(filename).is_absolute()
                or ".." in PurePosixPath(filename).parts
                or not SOURCE_FILE_RE.fullmatch(PurePosixPath(filename).name)
            ):
                raise LicenseBundleError(f"license manifest filename is unsafe: {key}")
            if filename.casefold() in folded_names:
                raise LicenseBundleError(f"duplicate license manifest package/file: {key} {filename}")
            folded_names.add(filename.casefold())
            filenames.append(filename)
            if not isinstance(source_file["origin"], str):
                raise LicenseBundleError(f"license manifest origin is malformed: {key}")
            digest = source_file["sha256"]
            if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
                raise LicenseBundleError(f"license manifest digest is malformed: {key}")
            if (
                not isinstance(source_file["byte_length"], int)
                or isinstance(source_file["byte_length"], bool)
                or source_file["byte_length"] < 0
            ):
                raise LicenseBundleError(f"license manifest byte length is malformed: {key}")
            text_digests.add(digest)
            mapping_count += 1
        if filenames != sorted(filenames, key=lambda item: (item.casefold(), item)):
            raise LicenseBundleError(f"license manifest files are reordered: {key}")
        if key in fallbacks:
            fallback = fallbacks[key]
            expected_files = [
                {
                    "byte_length": source_file["byte_length"],
                    "filename": source_file["path"],
                    "origin": (
                        f"upstream:{fallback['repository']}@{fallback['revision']}/"
                        f"{source_file['path']}"
                    ),
                    "sha256": source_file["sha256"],
                }
                for source_file in fallback["candidate_files"]
            ]
            expected_status = (
                "preserved-from-pinned-upstream"
                if expected_files
                else "no-candidate-file-in-exact-crate-or-pinned-upstream"
            )
            if files != expected_files:
                raise LicenseBundleError(f"license manifest fallback mapping disagrees: {key}")
            fallback_seen.add(key)
        else:
            expected_status = "preserved-from-exact-crate-package"
            if not files or any(
                source_file["origin"] != f"crate-package:{source_file['filename']}"
                for source_file in files
            ):
                raise LicenseBundleError(f"license manifest crate mapping is incomplete: {key}")
        if entry["file_status"] != expected_status:
            raise LicenseBundleError(f"license manifest file status disagrees: {key}")
        records[key] = entry
        actual_order.append(key)
    if actual_order != sorted(inventory) or set(records) != set(inventory):
        raise LicenseBundleError("license manifest package membership or ordering disagrees")
    if fallback_seen != set(fallbacks):
        raise LicenseBundleError("license manifest fallback membership disagrees")
    if document["mapping_count"] != mapping_count:
        raise LicenseBundleError("license manifest mapping count disagrees")
    if document["unique_text_count"] != len(text_digests):
        raise LicenseBundleError("license manifest unique text count disagrees")
    return records


def load_reviewed_manifest(
    root: Path,
    inventory: dict[tuple[str, str], InventoryPackage],
    fallbacks: dict[tuple[str, str], dict[str, Any]],
    manifest_path: Path | None = None,
) -> tuple[dict[str, Any], dict[tuple[str, str], dict[str, Any]]]:
    path = root / MANIFEST_PATH if manifest_path is None else manifest_path
    try:
        value = path.read_bytes()
    except OSError as error:
        raise LicenseBundleError(f"cannot read {MANIFEST_PATH}: {error}") from error
    document = parse_canonical_json(value, "license manifest")
    return document, validate_manifest(document, inventory, fallbacks)


def verify_reviewed_bundle(
    root: Path = ROOT,
    *,
    artifact_path: Path | None = None,
    manifest_path: Path | None = None,
) -> tuple[int, int, int]:
    inventory = parse_inventory(root)
    fallbacks = load_fallbacks(root)
    manifest, expected_records = load_reviewed_manifest(
        root, inventory, fallbacks, manifest_path
    )
    path = root / ARTIFACT_PATH if artifact_path is None else artifact_path
    try:
        value = path.read_bytes()
    except OSError as error:
        raise LicenseBundleError(f"cannot read {ARTIFACT_PATH}: {error}") from error
    packages, texts = parse_bundle(value)
    actual_keys: list[tuple[str, str]] = []
    referenced_texts: set[str] = set()
    for bundled in packages:
        package = bundled["package"]
        exact_keys(
            package,
            {
                "checksum",
                "declared_license",
                "file_count",
                "file_status",
                "name",
                "relationship",
                "source",
                "targets",
                "version",
            },
            "package record",
        )
        key = (package["name"], package["version"])
        if key in actual_keys:
            raise LicenseBundleError(f"duplicate license bundle package: {key}")
        actual_keys.append(key)
        if key not in inventory or key not in expected_records:
            raise LicenseBundleError(f"license bundle has an extra package: {key}")
        expected_package = inventory[key]
        expected_fields = {
            "checksum": expected_package.checksum,
            "declared_license": expected_package.declared_license,
            "name": expected_package.name,
            "relationship": expected_package.relationship,
            "source": expected_package.source,
            "targets": list(expected_package.targets),
            "version": expected_package.version,
        }
        for field, expected_value in expected_fields.items():
            if package[field] != expected_value:
                raise LicenseBundleError(f"license bundle {field} disagrees: {key}")
        files = bundled["files"]
        if package["file_count"] != len(files):
            raise LicenseBundleError(f"license bundle file count disagrees: {key}")
        actual_files: list[dict[str, Any]] = []
        folded_names: set[str] = set()
        for source_file in files:
            exact_keys(
                source_file,
                {"byte_length", "filename", "origin", "sha256", "text_id"},
                "file record",
            )
            filename = source_file["filename"]
            if (
                not isinstance(filename, str)
                or PurePosixPath(filename).is_absolute()
                or ".." in PurePosixPath(filename).parts
                or not SOURCE_FILE_RE.fullmatch(PurePosixPath(filename).name)
            ):
                raise LicenseBundleError(f"license bundle filename is unsafe: {key}")
            if filename.casefold() in folded_names:
                raise LicenseBundleError(f"duplicate license bundle package/file: {key} {filename}")
            folded_names.add(filename.casefold())
            digest = source_file["sha256"]
            if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
                raise LicenseBundleError(f"license bundle digest is malformed: {key}")
            if source_file["text_id"] != f"sha256:{digest}":
                raise LicenseBundleError(f"license bundle text identity disagrees: {key}")
            actual_files.append(
                {
                    "byte_length": source_file["byte_length"],
                    "filename": filename,
                    "origin": source_file["origin"],
                    "sha256": digest,
                }
            )
            referenced_texts.add(digest)
        expected_record = expected_records[key]
        if package["file_status"] != expected_record["file_status"]:
            raise LicenseBundleError(f"license bundle file status disagrees: {key}")
        if actual_files != expected_record["files"]:
            raise LicenseBundleError(f"license bundle mapping disagrees with manifest: {key}")
    if actual_keys != sorted(inventory) or set(actual_keys) != set(inventory):
        raise LicenseBundleError("license bundle package membership or ordering disagrees")
    if referenced_texts != set(texts):
        raise LicenseBundleError("license bundle has missing or unreferenced canonical text")
    counts = (len(packages), sum(len(item["files"]) for item in packages), len(texts))
    expected_counts = (
        manifest["package_count"],
        manifest["mapping_count"],
        manifest["unique_text_count"],
    )
    if counts != expected_counts:
        raise LicenseBundleError("license bundle counts disagree with manifest")
    return counts


def verify_sources(root: Path = ROOT, cargo_home: Path | None = None) -> tuple[int, int, int]:
    packages = build_packages_from_sources(root, cargo_home)
    expected_bundle = render_bundle(packages)
    expected_manifest = render_manifest(packages)
    try:
        actual_bundle = (root / ARTIFACT_PATH).read_bytes()
        actual_manifest = (root / MANIFEST_PATH).read_bytes()
    except OSError as error:
        raise LicenseBundleError(f"cannot read reviewed license artifact: {error}") from error
    if actual_bundle != expected_bundle:
        raise LicenseBundleError(f"{ARTIFACT_PATH} bytes disagree with exact package sources")
    if actual_manifest != expected_manifest:
        raise LicenseBundleError(f"{MANIFEST_PATH} bytes disagree with exact package sources")
    return verify_reviewed_bundle(root)


def write_atomic(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(value)
    temporary.replace(path)


def main(arguments: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    generate = subparsers.add_parser("generate")
    generate.add_argument("--output", type=Path, default=ROOT / ARTIFACT_PATH)
    generate.add_argument(
        "--manifest-output", type=Path, default=ROOT / MANIFEST_PATH
    )
    subparsers.add_parser("verify")
    subparsers.add_parser("verify-reviewed")
    parsed = parser.parse_args(arguments)
    try:
        if parsed.command == "generate":
            packages = build_packages_from_sources()
            bundle_value = render_bundle(packages)
            manifest_value = render_manifest(packages)
            write_atomic(parsed.output, bundle_value)
            write_atomic(parsed.manifest_output, manifest_value)
            counts = (
                len(packages),
                sum(len(item.files) for item in packages),
                len({source_file.digest for item in packages for source_file in item.files}),
            )
        elif parsed.command == "verify":
            counts = verify_sources()
        else:
            counts = verify_reviewed_bundle()
    except LicenseBundleError as error:
        print(f"third-party license bundle failed: {error}", file=sys.stderr)
        return 2
    print(f"third-party license bundle verified: packages={counts[0]} files={counts[1]} unique_texts={counts[2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
