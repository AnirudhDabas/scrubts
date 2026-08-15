# Mega C release-integrity contract

## Integrity layers

Mega C treats four facts as independent:

1. `SHA256SUMS` checks archive bytes against a checksum manifest. It does not
   authenticate a publisher.
2. A GitHub artifact attestation links an archive digest to a GitHub Actions
   build-provenance statement. It is not platform-vendor signing, a human
   identity proof, or compiler-output reproducibility.
3. An immutable GitHub release locks the published release tag and assets and
   gives consumers a release-membership attestation. A mutable draft is not an
   immutable release.
4. Apple Developer ID/notarization and Windows Authenticode are platform-vendor
   trust mechanisms. Mega C provides none of them for v0.1 and says so in every
   archive.

## Release state machine

The only approved sequence is:

`SOURCE COMMIT -> NATIVE BUILD -> BINARY SMOKE -> DETERMINISTIC PACKAGE ->`
`PACKAGE DIGEST -> BUILD ATTESTATION -> MULTI-PLATFORM ASSEMBLY ->`
`RELEASE MANIFEST -> SHA256SUMS -> DRAFT GITHUB RELEASE -> HUMAN PUBLISH ->`
`IMMUTABLE RELEASE`.

Preflight dispatch stops after assembly and uploads workflow artifacts. A
pushed release tag additionally attests the exact archives and creates or
updates one draft release. Automation never publishes it.

Before a human publishes v0.1, repository release immutability MUST be enabled.
Neither workflow source nor a draft proves that prerequisite is satisfied.

## Native matrix

| Artifact platform | Rust target | GitHub-hosted runner |
| --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | `windows-2025` |
| Linux x64 | `x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `macos-15` |
| macOS Intel | `x86_64-apple-darwin` | `macos-15-intel` |

Every job installs project toolchain Rust 1.97.1 explicitly and builds scrub
with `cargo +1.97.1 build --locked --release --target <target> -p scrub`.
`Cargo.lock` supplies exact dependency versions and registry checksums. The
matrix records the built target and actual runner label; it makes no additional
runtime-OS-minimum claim.

## Version and source identity

`crates/scrub/Cargo.toml` is the canonical package-version source.
`crates/scrub-report/Cargo.toml` must carry the same version for v0.1. A release
tag is a strict `v<scrub version>` string. For the current manifests the only
accepted tag is `v0.1.0`.

Tag mode rejects malformed or mismatching tags. Checkout HEAD, `github.sha`,
and the commit addressed by `refs/tags/<tag>` must be identical. Release
creation uses `gh release create --verify-tag --target <github.sha>` after that
check, so a missing tag cannot silently fall back to the default branch.

## Archive contract

Windows uses deterministic ZIP; Linux and macOS use deterministic tar+gzip.
Every archive has exactly this tree:

```text
scrub-vX.Y.Z-<target>/
    scrub | scrub.exe
    LICENSE
    THIRD_PARTY_NOTICES.md
    RELEASE-METADATA.json
```

Members are sorted. Timestamps, ownership fields, names, permissions, gzip
header metadata, and ZIP metadata are normalized. The Unix binary mode is 0755;
all other files are 0644. Generated JSON is UTF-8 with LF endings and a final
newline. Therefore identical binary, license, notice, metadata, and packager
inputs produce byte-identical archives. This is deterministic packaging, not a
claim of bit-for-bit reproducible compiler output.

Verification accepts that one canonical representation, not merely an archive
with equivalent extracted files. ZIP type, order, compression, platform,
attribute, timestamp, flag, and end-record fields must match packager output.
Tar verification reads each USTAR header and applies type, name, and declared
size bounds before payload bytes; PAX records are not part of the format. The
gzip envelope is one member with the packager's fixed header and no optional or
trailing data. This remains a bounded scrub package verifier, not a universal
ZIP/tar validator.

The strict `schemas/release-artifact-0.1.schema.json` document records only
stable package, source commit and tree state, target, digest, toolchain,
profile, signing, and limitation facts. Semantic validation checks internal
bytes against the binary digest, the repository `Cargo.lock`, expected
tag/version/source/target, safe relative names, exact members, normalized
archive metadata, and bounded archive/member sizes before decompression.

Native verification extracts only after structural validation, into a fresh
temporary directory. It then runs the extracted binary with `--help` and runs
`inspect --json` against a generated deterministic local fixture. Passing the
pre-package binary cannot satisfy this check.

## Aggregate contract

Assembly accepts exactly one valid archive for every matrix target. It rejects
missing, duplicate, unknown, or extra targets and any source, version, tag,
archive digest, internal metadata, Cargo.lock digest, or package-name
disagreement.

Release-output verification also inventories its complete input directories:
exactly four archives plus `release-manifest.json` and `SHA256SUMS` are allowed.
An archive added after assembly is a verification failure even when it is not
referenced by the manifest.

`release-manifest.json`, validated by
`schemas/release-manifest-0.1.schema.json`, identifies the mode, scrub version,
tag when applicable, source commit, exact expected target set, and archive,
binary, metadata, target, and packaging-schema identities for each row.

Tag and GitHub-dispatch packages require `source_tree_state: clean_commit`.
Local authoring packages may use `dirty_worktree` so a verification artifact
does not pretend that its bytes came only from the baseline commit.

`SHA256SUMS` covers all four downloadable archives and
`release-manifest.json`, sorted by filename in conventional
`<lowercase sha256>  <filename>` format with LF endings. It does not include
itself.

## GitHub workflow contract

Build jobs have `contents: read`. Tag-mode build jobs alone add
`id-token: write` and `attestations: write`; no OCI storage record is created,
so `artifact-metadata: write` is not granted. The tag-only attestation step uses
`actions/attest` pinned to v4.2.2 commit
`1e69f48acb82d1966a394da916b4c1698aa569d6` with an explicit `subject-path`
to the completed archive.

The assembly job has read-only repository permissions. The tag-only draft job
alone has `contents: write`. It refuses to update a published release, rejects
unexpected existing draft assets, and uploads exactly four archives,
`release-manifest.json`, and `SHA256SUMS`. Human inspection and publication are
outside automation.

All actions use immutable commits. No workflow uses `pull_request_target`,
untrusted pull-request release writes, moving action tags, or downloaded
floating executables.

## Consumer verification

After a future human publishes an immutable release, consumers may run:

```bash
gh release verify vX.Y.Z -R OWNER/REPO
gh release verify-asset vX.Y.Z ./PATH/TO/ARCHIVE -R OWNER/REPO
gh attestation verify ./PATH/TO/ARCHIVE -R OWNER/REPO
```

The first verifies that GitHub recognizes the release as immutable. The second
verifies that local archive bytes are an asset in that immutable release. The
third verifies GitHub/Sigstore artifact provenance for those archive bytes.
Local checksum verification is a separate comparison against `SHA256SUMS`.

Mega C authoring establishes none of the future GitHub results. Until an actual
tag workflow and human immutable publication occur, Linux/macOS packages,
GitHub build attestations, the draft release, release attestation, and immutable
release membership are **NOT YET ESTABLISHED**.
