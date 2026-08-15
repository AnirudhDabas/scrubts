# Mega C: release integrity

## Goal

Ship a repository-owned, testable release contract in which each native scrub
archive identifies its source and contents, the four archives assemble only as
a complete release set, tag builds attest the exact downloadable archives, and
automation stops at one draft GitHub release for human inspection and publish.

## Non-goals

- Publishing a release, creating or pushing a tag, or asserting that release
  immutability or a real attestation already exists.
- Compiler-output reproducibility, benchmarks, visual/README launch work,
  installers, package managers, SBOM generation, or platform signing secrets.
- New release targets beyond the four native x64/Apple Silicon targets in the
  milestone request.
- Changes to WaterLARP or the frozen Mega A/B evidence semantics. Mega B changes
  are limited to mechanically extending `proof_relevant_project` source scope.

## Sources / authority

- GitHub Docs at `81ade08c26f13325c0cde8a23cd3bfb85bd0778e`:
  immutable releases, release-integrity verification, artifact attestations,
  and GitHub-hosted runner labels.
- `actions/attest` v4.2.2,
  `1e69f48acb82d1966a394da916b4c1698aa569d6`.
- GitHub CLI v2.97.0 source revision
  `55dbb4dc6b7edb10b48e3d7fc5bccd32318d1b55` and its current online manual.
- Rust 1.97.1 source revision
  `8bab26f4f68e0e26f0bb7960be334d5b520ea452` and rustc target-support docs.
- Existing ADR 0004 is read-only human-owned input. Mega C implements its
  checksum/provenance boundary but explicitly defers its optional SBOM work.

Exact source records and limitations live in `research/sources.yaml`.

## Current state

Baseline `8683d086586117a74b1a78897f45861d1532e4cf` has Rust 1.97.1,
workspace package versions 0.1.0, `Cargo.lock`, SHA-pinned checkout and artifact
actions, and a proof source identity. It has no release packager, release
schemas, release workflow, draft assembly, or locally established release
package evidence. Nine unrelated untracked human-owned files remain outside
this milestone.

## Design

The release path is:

`source commit -> native build -> binary smoke -> deterministic package ->`
`package digest -> tag-only build attestation -> four-platform assembly ->`
`release manifest -> SHA256SUMS -> draft GitHub release -> human publish ->`
`immutable release`.

`tools/release.py` is the sole packaging contract. It reads the canonical scrub
version from `crates/scrub/Cargo.toml`, requires coordinated scrub-report
versioning, creates normalized ZIP (Windows) and tar+gzip (Unix) archives,
validates archive metadata and content relationships, assembles an exact
four-target manifest, and writes sorted LF-only checksums.

The workflow has dispatch preflight and pushed-tag modes. Both build and smoke
the four native artifacts with the pinned toolchain. Only the tag-mode native
build jobs receive OIDC/attestation writes and invoke `actions/attest`; only tag
mode can create or update a draft release. A separate minimally privileged job
owns draft release writes. No workflow step publishes a release or creates a
tag.

## Acceptance criteria

- Exactly the requested target/runner matrix is encoded without `*-latest`.
- A real release tag is exactly `v<scrub version>`, resolves to the workflow
  commit, and cannot be synthesized by release creation.
- Archives contain only the binary, `LICENSE`, `THIRD_PARTY_NOTICES.md`, and
  strict `RELEASE-METADATA.json` under one versioned target directory.
- Metadata distinguishes a clean workflow commit from a local dirty authoring
  worktree; tag mode rejects the latter.
- Identical packaging inputs produce byte-identical archives. This claim is
  deterministic packaging only.
- Verification extracts to a clean temporary directory, validates digests and
  metadata, and smokes the extracted binary with help plus one local inspect.
- Assembly rejects incomplete, duplicate, unknown, extra, or disagreeing
  packages and emits strict `release-manifest.json` plus `SHA256SUMS`.
- The attestation subject path is the exact archive later uploaded as a release
  asset.
- Documentation distinguishes checksums, build provenance, immutable release
  membership, and platform-vendor signing.
- Proof identity includes every new release-bearing source path, while the
  public claim ledger remains at 16 claims.

## Implementation steps

1. Record current primary release sources and freeze this plan/spec.
2. Add strict Draft 2020-12 artifact and aggregate-manifest schemas.
3. Implement the standard-library release tool and contract-focused negative
   tests.
4. Add the two-mode, SHA-pinned four-target workflow and operator documentation.
5. Extend proof-source scope tests for the new release paths without adding an
   unestablished release claim.
6. Run focused schema/tool tests, a real local Windows release package smoke,
   synthetic four-target assembly, full repository gates, and one adversarial
   Mega C self-review.

## Validation

- `python -m unittest tools.tests.test_release tools.tests.test_prove`
- Draft 2020-12 schema checks through the release tests.
- Two-package deterministic archive digest regression.
- Local `cargo +1.97.1 build --locked --release --target
  x86_64-pc-windows-msvc -p scrub`, followed by package verification and smoke.
- Synthetic exact four-target assembly and checksum verification.
- Workflow YAML parse and immutable action-pin source checks.
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `just check`
- `just prove`
- `git diff --check`

## Risks / open questions

- Linux and both macOS packages, tag attestations, draft-release behavior, and
  immutable publication require future GitHub execution and remain not yet
  established locally.
- GitHub-hosted runner images change while labels remain stable; package
  metadata records the target and exact Rust/Cargo versions, while the workflow
  run remains the authority for the actual tested image.
- Deterministic archives do not imply deterministic compiler output.
- The current v0.1 archives intentionally provide neither Apple Developer
  ID/notarization nor Windows Authenticode. GitHub attestations do not replace
  those vendor trust systems.

## Outcome

Implemented the repository-owned packager, both strict schemas, the exact
four-target workflow, consumer/operator documentation, proof-source extension,
and standard `--help`/`--version` release smoke surface. No public claim was
added; the ledger remains at 16 claims.

Source intake on 2026-08-15 confirmed the revisions recorded above, including
`actions/attest` v4.2.2 at
`1e69f48acb82d1966a394da916b4c1698aa569d6`. Current optional diagnostics were
`cargo-audit` 0.22.2, `cargo-deny` 0.20.2, and `cargo-auditable` 0.7.5. None was
installed or made a release dependency, and no SBOM was added because Mega C
has no reviewed SBOM consumer or policy gate.

Focused validation established:

- 42 release/proof unit tests, including Draft 2020-12 schema validation,
  negative contract cases, synthetic four-target assembly, deterministic ZIP
  and tar+gzip regressions, checksum verification, proof scope, and workflow
  pin/behavior checks;
- workflow YAML parsing with five jobs and exact immutable action pins;
- a real Rust 1.97.1 Windows x64 `--locked --release` build;
- native help and offline inspect smoke, followed by clean-temp extraction and
  the same two smokes against the archived binary;
- two byte-identical Windows packages with archive SHA-256
  `2991a464f50646dc924e102920e64471506fa609a9d3712b327eba22fd520324`
  and binary SHA-256
  `3c399083ae3c81c36cc862f694e495c1d958567a1fe3770c621940bdc9bb04fd`.
  The ignored local metadata honestly records `dirty_worktree` and therefore
  is not a publishable release artifact.

The full gates passed: `cargo fmt --check`, clippy with warnings denied,
`cargo test --workspace`, `just check`, `just prove` with all 16 gates and
`PROOF_COMPLETE`, and `git diff --check`. The first optimized build attempt ran
out of disk and the first workspace-test link attempts hit MSVC PDB limits.
Only ignored Cargo outputs were removed; the optimized build then passed, and
the complete workspace suite passed with `CARGO_BUILD_JOBS=1` before the exact
`just check` wrapper passed from the resulting artifacts.

The initial focused self-review added archive and member-size bounds. A later
independent targeted review showed that the verifier still accepted special ZIP
member types, duplicate JSON keys, hidden/noncanonical ZIP, PAX, and gzip
metadata, and an archive added after assembly; compressed-tar enumeration also
traversed an oversized declared payload before applying its bound. The bounded
verifier fix now checks canonical ZIP structure and member types, uses strict
JSON object parsing, validates each USTAR header before payload consumption,
accepts one canonical gzip member with no PAX or trailing data, and inventories
the exact release directories. Focused regressions reproduce each accepted
case. These changes do not alter tag/source architecture, release targets,
attestation, publication, signing, claim, WaterLARP, or Mega A/B semantics.

GitHub artifact attestations, the draft release, the release attestation,
repository immutability configuration, human publication, and immutable public
release membership remain **NOT YET ESTABLISHED** pending future tag-mode
GitHub execution and operator action.

External preflight run `31888740599` passed native build, package, and smoke on
all four targets. Assembly then rejected the Windows artifact because checkout
had converted the committed root `Cargo.lock` from LF to CRLF.

External preflight run `31890505072` again passed native build, package, and
smoke on all four targets. The prior Cargo.lock blocker was resolved. Assembly
then rejected the Windows artifact because checkout had converted the committed
root `LICENSE` from LF to CRLF. Downloaded artifacts also proved the same latent
checkout conversion for `THIRD_PARTY_NOTICES.md`.

These remote executions exposed platform checkout-byte differences in the
repository-owned exact-byte release inputs. The checkout contract now marks
only `Cargo.lock`, `LICENSE`, and `THIRD_PARTY_NOTICES.md` as exact-byte release
sources, and clean-commit packaging requires each working byte sequence to equal
its exact blob at `source_commit`. Dirty-worktree packaging retains its existing
semantics. Aggregate assembly remains **NOT YET ESTABLISHED**.
