# Milestone 2A: Unicode Default_Ignorable_Code_Point reporting

## Goal

Add one deterministic scanner that classifies an artifact against the Unicode
17.0.0 `Default_Ignorable_Code_Point` (DICP) property and reports bounded,
auditable occurrence locations under
`docs/specs/unicode-default-ignorable.md`.

## Non-goals

- Bidi-control analysis as its own mechanism.
- Normalization or confusable analysis.
- Sanitization, removal, or any other input transformation.
- C2PA inspection.
- Statistical watermark detection, including Claude-specific detection.
- WaterLARP experiments.
- A generic scanner, registry, plugin, factory, manager, or engine architecture.
- A new crate.

## Sources / authority

- `docs/specs/unicode-default-ignorable.md`
- Unicode Standard Annex #44, Unicode Character Database, for property
  semantics.
- Unicode 17.0.0 `DerivedCoreProperties.txt`, pinned in
  `research/sources.yaml`, for normative DICP membership.
- Unicode Technical Standard #55, Unicode Source Code Handling, for
  interpretation and security guidance, not membership.
- Unicode Technical Standard #39, Unicode Security Mechanisms, only as a
  boundary: future confusable analysis is outside this milestone.
- `docs/specs/report-schema.md`
- `docs/adr/0001-evidence-status-model.md`
- `docs/adr/0002-statistical-detectors-are-controlled.md`
- `docs/adr/0003-source-authority-and-conformance.md`

The pinned data has SHA-256
`24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08`,
is 1,134,783 bytes, and contains 27 explicit DICP ranges covering 4,174
code points. Its local Unicode License V3 text has SHA-256
`e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

## Current state

Milestone 1 provides the schema 0.1 report model and a single-file CLI that
reports no findings. The Unicode 17.0.0 data and license are present locally,
and the source ledger and scanner contract are prepared. No Unicode scanner or
DICP fixture currently exists, and `CONFORMANCE.md` does not claim DICP support.

## Design

Keep the implementation in the existing crates. Represent the 27 pinned DICP
ranges directly and explicitly; do not add runtime networking, a general UCD
parser, or future-scanner abstractions. Validate the complete artifact as UTF-8
before membership classification. Scan Unicode scalar values in input order,
count every matching occurrence, and retain only the first 256 locations with
their zero-based byte and scalar-value offsets. Report location truncation
explicitly.

The finding mechanism and version identify DICP and Unicode 17.0.0. Human and
machine wording remains neutral and does not aggregate this observation into a
security, provenance, AI-authorship, or watermark claim. Inspection remains
read-only.

## Acceptance criteria

- The membership table agrees exactly with all 27 explicit DICP ranges and
  4,174 code points in the pinned data.
- Valid UTF-8 with at least one member reports `PRESENT`; valid UTF-8 with none
  reports `ABSENT`; malformed or incomplete UTF-8 reports `INVALID`.
- No malformed input path reports `ABSENT`, and no invalid suffix permits a
  prefix-only `PRESENT` or `ABSENT` result.
- Byte and Unicode scalar-value offsets follow the scanner contract, including
  around multibyte and supplementary-plane values.
- Every occurrence contributes to the total count. At most the first 256
  locations are retained, and truncation is true exactly when the count exceeds
  256.
- Findings and retained locations have deterministic ordering.
- The input bytes are unchanged after inspection.
- Reports use neutral property wording and make none of the excluded security,
  provenance, authorship, watermark, or removability claims.
- `CONFORMANCE.md` claims support only after fixtures and all required tests
  pass, with Unicode 17.0.0 and the pinned data recorded as authority.

## Implementation steps

1. Source contract: audit governing specs and ADRs, pin the Unicode data and
   license provenance in `research/sources.yaml`, and approve
   `docs/specs/unicode-default-ignorable.md`.
2. Fixtures: before production code, add minimal valid UTF-8 fixtures for
   absence, singleton/range boundaries, multibyte and supplementary-plane
   offsets, repeated occurrences, and 256/257-location bounds. Add malformed
   leading, continuation, overlong, surrogate, out-of-range, and truncated UTF-8
   cases with explicit expected statuses and evidence.
3. Minimal implementation: add direct Unicode 17.0.0 DICP membership and one
   scanner path in the existing crates, then connect it to single-file
   inspection without adding a generic scanner architecture or a new crate.
4. Adversarial/property tests: exhaustively compare scalar membership against
   the 27 pinned ranges; test all range boundaries, offset accounting,
   determinism, hostile UTF-8, repeated values, evidence bounds, and read-only
   behavior.
5. Conformance update: after the implementation and tests pass, add the exact
   authority, version, fixture status, deviations, and check date to
   `CONFORMANCE.md`.

## Validation

- Recompute SHA-256 and byte size for both local Unicode files.
- Parse the pinned data independently and verify 27 explicit DICP ranges and
  4,174 code points.
- `cargo test --offline -p scrub --test unicode_default_ignorable`
- `cargo test --offline --workspace`
- `just check`
- `git diff --check`
- `git status --short`
- Manually inspect machine and human output for neutral wording and verify the
  input hash is unchanged before and after inspection.

## Risks / open questions

- DICP membership can change in later Unicode releases. This milestone must not
  substitute host-library Unicode data or silently advance beyond 17.0.0.
- DICP includes characters with legitimate language, shaping, emoji, and
  formatting uses. Presence is not a security or intent classification.
- The report core currently stores evidence as name/value strings. Fixture
  expectations must pin one deterministic encoding for total count, retained
  locations, and truncation before production scanner code is written; this
  does not require a generic evidence framework.

## Outcome

The source ledger and public scanner contract are prepared. Fixtures,
implementation, tests, and the DICP conformance claim remain pending and must
proceed in the order above.
