# Milestone 3A: Unicode 17.0.0 Bidi_Control source authority and reporting contract

## Goal

Pin the authoritative Unicode 17.0.0 sources for `Bidi_Control` membership and
interpretation, and define the future `unicode.bidi_control` reporting contract.
This milestone adds documentation and source provenance only. It does not claim
that scrub.ts can inspect this mechanism in production.

## Non-goals

- Rust, Cargo, dependency, report-schema, or implementation changes.
- A production scanner or production fixtures.
- Trojan Source detection or contextual source-code vulnerability analysis.
- UAX #9 reordering, normalization, or confusable detection.
- Sanitization, removal, or any other input transformation.
- File-type detection, C2PA inspection, or watermark detection.
- A shared streaming-decoder refactor.
- A scanner trait, registry, plugin architecture, generic Unicode framework, or
  other future-oriented abstraction.
- A production conformance claim in `CONFORMANCE.md`.

## Sources / authority

- Unicode Standard Annex #44, Unicode Character Database, for property and data
  semantics and Unicode version stability.
- Unicode 17.0.0 `PropList.txt`, locally pinned at
  `research/library/unicode/17.0.0/PropList.txt`, as the sole membership
  authority.
- Unicode Standard Annex #9, Unicode Bidirectional Algorithm, Revision 51 for
  Unicode 17.0.0, for directional-formatting character names, abbreviations,
  and behavior.
- Unicode Technical Standard #55, Unicode Source Code Handling, for security
  and display interpretation, not membership.
- Unicode License V3 at
  `research/library/unicode/17.0.0/UNICODE-LICENSE.txt`.
- `docs/SOURCE_AUTHORITY.md`, `docs/specs/report-schema.md`, and
  `docs/adr/0001-evidence-status-model.md` for project evidence semantics.
- `docs/specs/unicode-default-ignorable.md` for established deterministic
  Unicode reporting conventions where this mechanism deliberately aligns.

Independent local verification on 2026-08-11 established:

- `PropList.txt`: 145,465 bytes; SHA-256
  `130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd`;
- `Bidi_Control`: 4 explicit records covering 12 code points;
- canonical six-digit `START..END` plus LF serialization: 60 bytes; SHA-256
  `217873f8bf2ca674f32afff23b3dc5fd81e4b55b5f6aa978c63417ad29f22674`;
- Unicode License V3: 1,995 bytes; SHA-256
  `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

## Current state

The production CLI reports Unicode 17.0.0 `Default_Ignorable_Code_Point`
findings and explicitly lists bidi-control analysis as unevaluated. The pinned
local Unicode research corpus already contains `PropList.txt` and the Unicode
License V3 text, but the source ledger has no `PropList.txt` binding and there is
no approved Bidi_Control reporting contract. No Bidi_Control fixture, scanner,
or conformance claim exists.

## Design

Add `docs/specs/unicode-bidi-control.md` as the complete reporting contract and
record the UAX #44, `PropList.txt`, UAX #9, and UTS #55 roles in
`research/sources.yaml`. Membership comes only from the four pinned
`Bidi_Control` records. UAX #9 supplies the 12 canonical abbreviations; UTS #55
supplies interpretation guidance.

The contract aligns status, offset, complete-input UTF-8 validation, count, and
256-location retention semantics with DICP while keeping a distinct mechanism
identifier and evidence. It records overlap between the two Unicode properties
without collapsing their independent findings.

## Acceptance criteria

- The raw pinned file size and SHA-256 are independently recomputed.
- An independent parser derives exactly these ordered ranges: `061C`,
  `200E..200F`, `202A..202E`, and `2066..2069`.
- The parser derives 4 records and 12 code points.
- Canonical serialization independently produces 60 bytes and semantic SHA-256
  `217873f8bf2ca674f32afff23b3dc5fd81e4b55b5f6aa978c63417ad29f22674`.
- UAX #9 confirms all 12 code point/abbreviation identities in the specification.
- `PRESENT`, `ABSENT`, and `INVALID` have the complete-input semantics in the
  approved contract; invalid UTF-8 cannot become `ABSENT` or retain prefix
  occurrence evidence.
- Evidence specifies canonical code point and abbreviation, byte and scalar
  offsets, total count, first-256 retention, and explicit truncation without
  emitting raw controls.
- The contract lists the required neutral non-claims and independent DICP
  overlap semantics.
- `CONFORMANCE.md` remains unchanged because no production support exists.
- No Rust, Cargo, dependency, report-schema, fixture, or Git history/staging
  mutation occurs.

## Implementation steps

1. Independently hash and parse the pinned `PropList.txt`; derive the ordered
   membership, extent, canonical serialization, and semantic digest.
2. Verify the 12 control abbreviations against official UAX #9 Revision 51 and
   verify the local Unicode license artifacts.
3. Add the pinned data and interpretation authorities to
   `research/sources.yaml`.
4. Define the status, evidence, overlap, non-claim, and implementation-boundary
   contract in `docs/specs/unicode-bidi-control.md`.
5. Parse the source ledger, run the repository quality gate and whitespace
   check, then inspect the complete diff and worktree state.

## Validation

- PowerShell SHA-256 and byte-size checks for `PropList.txt` and both local
  Unicode License V3 copies.
- An independent PowerShell parser for `Bidi_Control` records, code-point
  extent, canonical serialization, byte length, and SHA-256.
- Official UAX #9 Revision 51 HTML verification of all 12 code point and
  abbreviation pairs.
- Local YAML parsing of `research/sources.yaml`.
- `git diff --check`
- `just check`
- `git diff --stat`
- `git diff`
- `git status --short`

## Risks / open questions

- Property membership is versioned data and must not silently advance beyond
  Unicode 17.0.0.
- Directional-formatting controls have legitimate uses; property presence alone
  cannot support a security or intent classification.
- `Bidi_Control` is a strict subset of `Default_Ignorable_Code_Point` in
  Unicode 17.0.0: all 12 `Bidi_Control` code points are DICP members, while DICP
  contains many additional code points. The two properties answer distinct
  questions and must remain independent findings.
- Milestone 3C must decide whether DICP and Bidi_Control should consume one
  shared validated scalar stream. That decision must be earned by the smallest
  production implementation and must not create a generic scanner framework.

## Outcome

Milestone 3A established the source and reporting contract without production
support. `research/sources.yaml` now records UAX #44 semantics, the pinned
Unicode 17.0.0 `PropList.txt` membership data, UAX #9 Revision 51 identities and
behavior, and UTS #55 interpretation guidance. The new specification fixes the
mechanism identifier/version, complete-input status semantics, bounded evidence,
independent DICP overlap, neutral non-claims, and the Milestone 3C streaming
architecture question.

Independent parsing reproduced all expected source facts: 145,465 raw bytes,
raw SHA-256
`130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd`,
4 records, 12 code points, 60 canonical bytes, and semantic SHA-256
`217873f8bf2ca674f32afff23b3dc5fd81e4b55b5f6aa978c63417ad29f22674`.
The two local Unicode License V3 copies are byte-identical and have the pinned
SHA-256. Official UAX #9 Revision 51 confirmed all 12 specified abbreviations.
No source conflict or ambiguity blocks later fixture work.

The source ledger parsed successfully with local PyYAML. `git diff --check`
passed with only Git's LF-to-CRLF working-copy notice, and a separate scan found
no trailing whitespace in the two new files. `just check` passed formatting,
clippy with warnings denied, and all 30 existing tests. The final diff and
worktree inspection found only the three milestone files plus pre-existing
unrelated untracked seed documents.

No Rust, Cargo, dependency, report-schema, production fixture, scanner,
`CONFORMANCE.md`, staging, or Git history change was made.
