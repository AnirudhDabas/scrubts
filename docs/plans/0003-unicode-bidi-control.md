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

# Milestone 3B: Unicode 17.0.0 Bidi_Control fixture oracle

## Goal

Freeze a licensed compact membership oracle and adversarial test-only corpus
for the approved `unicode.bidi_control` contract before production scanning is
implemented.

## Non-goals

- Production, CLI, Cargo, dependency, report-schema, or conformance changes.
- A shared UTF-8 decoder or refactor of the DICP production scanner.
- UAX #9 reordering, scope matching, Trojan Source detection, security scoring,
  intent classification, normalization, confusables, or content transformation.
- Generic Unicode or scanner architecture.

## Sources / authority

Milestone 3A's pinned authorities and reviewed semantics remain unchanged. The
compact fixture is licensed under the existing checked-in Unicode License V3
and binds only the four `Bidi_Control` records from Unicode 17.0.0
`PropList.txt`. UAX #9 Revision 51 supplies the explicit 12-entry abbreviation
table.

## Current state

The reporting contract exists, but there is no Bidi_Control fixture oracle or
adversarial corpus. Production does not support the mechanism and
`CONFORMANCE.md` correctly makes no support claim.

## Design

Add one compact source extract and a test-only Rust oracle independent of
future production membership code. Keep the four ranges and 12 identities
explicit. Construct named byte fixtures deterministically in test support and
test their expected status, complete-input UTF-8 semantics, input-order
locations, first-256 retention, evidence encoding, and artifact identities.

The corpus covers negatives, each property member, offset divergence,
structure-shaped sequences without structural interpretation, complete
membership and DICP overlap, 256/257 occurrences, valid and malformed UTF-8 at
the 65,536-byte read boundary, and distinct malformed UTF-8 forms. Invalid
fixtures retain no prefix property evidence.

## Acceptance criteria

- The compact source parses to four sorted, non-overlapping ranges and 12 code
  points, with exactly 60 canonical bytes and the reviewed semantic digest.
- The explicit abbreviation table is bijective over the property membership.
- If the ignored full `PropList.txt` is present, its pinned bytes and digest are
  verified and its parsed property records exactly match the compact fixture.
- The committed semantic digest test remains unconditional when the ignored
  research corpus is absent.
- Every required corpus category has a descriptive case whose expected
  evidence is independently checked against the test oracle.
- Boundary and invalid artifacts have frozen full byte lengths and SHA-256
  identities; malformed boundary input discards valid-prefix observations.
- No production, Cargo, report, conformance, unrelated seed, staging, or Git
  history change occurs.

## Implementation steps

1. Add the licensed compact `Bidi_Control` extract, reusing the existing
   Unicode License V3 fixture.
2. Add explicit test-only range, identity, evidence, and corpus definitions.
3. Add fixture-only source-binding, corpus, boundary, malformed-input,
   truncation, serialization, non-mutation, and DICP-overlap tests.
4. Run the narrow fixture target, formatting, lint, workspace tests, repository
   quality gate, whitespace checks, and final changed-path audit.

## Validation

- `cargo test -p scrub --test unicode_bidi_control_fixtures`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
- `just check`
- `git diff --stat`
- `git diff`
- `git status --short`
- `git diff --cached --name-only`

## Risks / open questions

- Milestone 3C still owns the smallest streaming-decoder design decision. These
  fixtures constrain observable behavior without choosing that architecture.
- The two Unicode properties remain independent claims even though all 12
  Bidi_Control scalars are DICP members.

## Outcome

Milestone 3B added a licensed compact Unicode 17.0.0 `Bidi_Control` extract,
an explicit test-only four-range and 12-identity oracle, and a 35-case
adversarial corpus. Eleven fixture-only tests bind the compact data to the
60-byte canonical serialization and semantic digest, prove exact parity with
the locally available pinned `PropList.txt`, and freeze status, offsets,
evidence serialization, DICP overlap, retention limits, boundary behavior,
malformed-input evidence discard, full artifact identities, determinism, and
input non-mutation.

The narrow fixture target passed all 11 tests. `cargo fmt --check`, clippy with
workspace/all-target warnings denied, all 41 workspace tests, `git diff
--check`, and `just check` passed. Final path inspection found no changes under
production or report source, Cargo files, `CONFORMANCE.md`, or unrelated seed
documents. The Git index remained empty. No source contradiction or scientific
question blocks Milestone 3C; its streaming-decoder architecture decision
remains intentionally open.

# Milestone 3C-1: Shared incremental UTF-8 scalar decoding

## Goal

Extract the production DICP scanner's incremental UTF-8 validation, bounded
carry, and original byte/scalar offset tracking into one small internal
component. Preserve all existing DICP findings, evidence, CLI output, artifact
identity, and single-pass inspection behavior.

## Non-goals

- Production `unicode.bidi_control` membership, findings, CLI wording, or
  conformance claims.
- A scanner trait, registry, plugin, generic Unicode framework, or other
  future-mechanism architecture.
- Whole-file buffering, a second artifact read, dependencies, unsafe code, or
  changes to report schema, hashing, byte counting, or the 65,536-byte buffer.

## Sources / authority

- `std::str::from_utf8` and `Utf8Error::{valid_up_to,error_len}` define the
  implementation semantics for valid prefixes, incomplete suffixes, and
  definite malformed input.
- `docs/specs/unicode-default-ignorable.md` governs preserved DICP status and
  offset semantics.
- `docs/specs/unicode-bidi-control.md` and the frozen Milestone 3B fixtures
  establish that a second deterministic Unicode consumer is approved, while
  remaining outside production scope here.
- `docs/specs/report-schema.md`, `docs/SOURCE_AUTHORITY.md`, and
  `CONFORMANCE.md` constrain evidence and support claims.

## Current state

`unicode_default_ignorable::Inspection` owns UTF-8 carry/validation and scalar
offset accounting together with DICP membership, counting, location retention,
and finding construction. The existing inspection loop reads one 65,536-byte
buffer, then hashes, scans, and counts the same returned byte slice.

## Design

Add one private module containing an incremental decoder state and a scalar
observation value. Each pushed byte slice is validated with safe
`std::str::from_utf8`; verified scalar values are delivered in input order with
zero-based original byte and Unicode scalar-value offsets. Retain only an
incomplete UTF-8 suffix, bounded to three bytes. Definite malformed input makes
the decoder terminal; an incomplete suffix becomes invalid only at finish.

DICP owns the decoder and observes its scalar events. DICP remains solely
responsible for property membership, occurrence counts, first-256 retention,
evidence serialization, and final finding semantics. Invalid final decoding
continues to discard all valid-prefix property evidence. The outer inspection
loop continues hashing and counting every byte after decoder invalidity.

## Acceptance criteria

- Every existing DICP and CLI test retains its current semantics.
- Whole, short, one-byte, multibyte-splitting, and 65,536-boundary partitions
  produce identical ordered scalar observations.
- Two-, three-, and four-byte scalars work across every legal split and pending
  carry never exceeds three bytes.
- Lone continuation, overlong, and surrogate encodings are definite errors;
  incomplete EOF is invalid only on finish.
- Byte and scalar offsets remain exact for ASCII, BMP, and supplementary-plane
  scalar values.
- The existing read buffer, single pass, hashing, byte counting, report schema,
  CLI output, dependencies, and Bidi production status do not change.

## Implementation steps

1. Add the focused internal decoder and its scalar observation contract.
2. Adapt DICP inspection to consume observations and remove its decoder fields.
3. Add compact decoder unit tests for partition invariance, carry/error
   semantics, offset divergence, short chunks, and repeated determinism.
4. Run focused DICP/CLI checks, full repository gates, and inspect the complete
   diff and Git state.

## Validation

- `cargo test -p scrub --bin scrub utf8_stream`
- `cargo test -p scrub --test unicode_default_ignorable`
- `cargo test -p scrub --test unicode_default_ignorable_fixtures`
- `cargo test -p scrub --test cli`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
- `just check`
- `git diff --stat`
- `git diff`
- `git status --short`
- `git diff --cached --name-only`

## Risks / open questions

- The decoder must not confuse `error_len() == None` with malformed input; only
  the former may be carried, and only until final EOF.
- The callback API must remain a scalar-delivery boundary rather than grow into
  a scanner framework.
- Milestone 3C's later Bidi production work must still decide how both property
  observers are wired to one decoder without changing independent finding
  semantics.

## Outcome

Milestone 3C-1 added one private `utf8_stream` module. Its `Decoder::push`
accepts each actual inspection byte slice and a fallible scalar observer, and
emits each verified `char` with its zero-based original byte and scalar-value
offset. The decoder owns safe `std::str::from_utf8` validation, a maximum
three-byte incomplete suffix, scalar/byte offset state, terminal malformed
input state, and incomplete-EOF invalidation in `finish`.

The DICP inspection state now owns this decoder and only performs DICP
membership, occurrence counting, first-256 location retention, evidence
serialization, and finding construction. Its final `INVALID` path remains the
sole output after malformed or incomplete UTF-8 and retains no valid-prefix
property evidence. The outer 65,536-byte read loop still hashes, scans, and
counts the same returned bytes in one pass and continues through the complete
artifact after decoder invalidity.

Six decoder unit tests cover whole and 1/2/3/7-byte chunks, deliberate
multibyte splits, a split at byte 65,536, every legal split of two-/three-/
four-byte scalars, the three required definite malformed forms, malformed input
becoming definite across chunks, incomplete EOF, ASCII/BMP/supplementary offset
divergence, bounded carry, and repeated deterministic execution.

The focused decoder target, both DICP targets, and compiled CLI target passed.
`cargo fmt --check`, Clippy with workspace/all-target warnings denied, all 47
workspace tests, `git diff --check`, and `just check` also passed. Existing hard
DICP fixtures retained exact status/evidence semantics for the 65,536-byte
valid and malformed boundaries, prefix-evidence discard, complete artifact
SHA-256/length after invalidity, and 256/257 occurrence retention. No subsequent
Bidi production blocker was found; later wiring must preserve one decoder pass
feeding independent property observers.

# Milestone 3C-2: Production Unicode 17.0.0 Bidi_Control inspection

## Goal

Emit an independent `unicode.bidi_control` finding from the production CLI for
every inspected artifact, using the approved Unicode 17.0.0 source contract and
all 35 frozen fixture cases while retaining exactly one incremental UTF-8 decode
pass.

## Non-goals

- UAX #9 rendering, reordering, scope, pairing, or balance analysis.
- Trojan Source, intent, spoofing, or source-code vulnerability classification.
- Normalization, confusables, sanitization, rewriting, metadata, C2PA,
  watermark detection, Claude detection, or WaterLARP.
- A detector trait, registry, plugin, generic Unicode engine, new crate,
  dependency, async path, unsafe code, whole-file buffer, or second read.
- A Bidi conformance claim in `CONFORMANCE.md`.

## Sources / authority

- `docs/specs/unicode-bidi-control.md` and the frozen Milestone 3B oracle govern
  membership, identities, status, evidence, and neutral interpretation.
- Unicode 17.0.0 `PropList.txt` contributes exactly four ranges and 12 members;
  UAX #9 Revision 51 contributes the 12 canonical abbreviations.
- `docs/specs/unicode-default-ignorable.md` governs the existing independent
  DICP finding and its invalid-input convention.
- `utf8_stream::Decoder`, reviewed in Milestone 3C-1, remains the sole UTF-8
  validation and scalar-offset source.
- `docs/specs/report-schema.md` preserves schema 0.1 and canonical ordering.

## Current state

The artifact loop reads one 65,536-byte slice at a time and feeds that slice to
SHA-256, DICP inspection, and byte counting. DICP currently owns the shared
decoder. Production emits one DICP finding and still describes Bidi controls as
unevaluated. The Bidi source contract and independent 35-case fixture oracle
exist only in test support.

## Design

Move ownership of the existing decoder to the concrete inspection coordinator
in `main`. Each validated `ScalarObservation` is delivered in order to one DICP
state and one Bidi state. The states independently own only property membership,
counting, first-256 retention, and finding construction. At EOF, the decoder's
single validity result is passed to both states; invalidity causes each to emit
its own `INVALID` finding with the established UTF-8 evidence and no prefix
property evidence.

Represent Bidi membership and identity as one explicit sorted 12-entry table.
No production source parser or inference from DICP, categories, names, or bidi
classes is introduced. Let the report model sort the two findings and each
finding's evidence under its existing canonical ordering.

## Acceptance criteria

- Valid UTF-8 with a Bidi member is `PRESENT`; valid UTF-8 without one is
  `ABSENT`; malformed or incomplete UTF-8 is `INVALID` for both Unicode
  findings with all prefix property evidence discarded.
- Exactly one decoder consumes each artifact slice, and hashing/counting
  continue over every successful read after terminal invalidity.
- Production membership equals the frozen four-range oracle over every Unicode
  scalar value and has exactly 12 members, with exact UAX #9 abbreviations.
- All 35 frozen fixtures pass through the compiled CLI with exact Bidi evidence,
  artifact identity, ordering, and non-modification behavior.
- ZWJ remains DICP `PRESENT` and Bidi `ABSENT`; all 12 controls independently
  make both findings `PRESENT`.
- First-256 retention, valid and malformed 65,536-byte boundary behavior, and
  repeated real-CLI byte determinism are explicitly tested.
- Human output is control-safe and neutral; the remaining limitation wording no
  longer describes Bidi observation as unevaluated.
- No schema, conformance, Cargo, dependency, unrelated documentation, Git index,
  or history change occurs.

## Implementation steps

1. Add the concrete Bidi observation/finding module and exhaustive production
   membership tests.
2. Place the existing decoder above both concrete Unicode states and fan out
   each scalar observation without adding a generalized observer framework.
3. Add the 35-case compiled-CLI integration target and focused overlap,
   retention, boundary, invalid-prefix, non-modification, and determinism
   assertions.
4. Update existing DICP and CLI expectations for the second independent finding
   and update truthful limitation/human-output wording.
5. Run focused and full validation, inspect all changed paths and Git state, and
   record the measured outcome here.

## Validation

- `cargo test -p scrub --bin scrub`
- `cargo test -p scrub --test unicode_bidi_control`
- `cargo test -p scrub --test unicode_bidi_control_fixtures`
- `cargo test -p scrub --test unicode_default_ignorable`
- `cargo test -p scrub --test unicode_default_ignorable_fixtures`
- `cargo test -p scrub --test cli`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
- `just check`
- `git diff --stat`
- `git diff`
- `git status --short`
- `git diff --cached --name-only`

## Risks / open questions

- Moving decoder ownership must not create a second validation path or change
  DICP offsets, evidence, invalid-prefix suppression, or read-loop behavior.
- Bidi membership must stay independent even though every member overlaps DICP;
  U+200D and range neighbors are important regression boundaries.
- Human rendering currently labels every finding as DICP and must become an
  explicit two-mechanism presentation without rendering the underlying control.

## Outcome

Milestone 3C-2 added one focused production Bidi module with an explicit sorted
12-entry code-point/UAX #9 abbreviation table. Its concrete state independently
counts every property occurrence and retains the first 256 canonical locations.
Valid input emits `PRESENT` or `ABSENT` with the frozen three evidence names;
invalid or incomplete UTF-8 emits `INVALID` with the existing
`utf8_validation` convention and no prefix property evidence.

The artifact loop now owns the one Milestone 3C-1 `Decoder`. Every validated
`ScalarObservation` is delivered in order to the concrete DICP and Bidi states.
The same successful 65,536-byte read slice still feeds SHA-256 and decoding,
then contributes to full byte counting. Terminal decoder invalidity does not
stop later reads, hashing, or counting. One final decoder validity result is
shared by both findings, while membership, count, retained locations, and
finding construction remain independent.

Production membership matched the frozen four-range oracle for every Unicode
scalar value from U+0000 through U+10FFFF, excluding surrogates, with exactly 12
members. Explicit identity and neighbor assertions confirmed all abbreviations
and rejected U+200D, U+2065, U+206A, and every tested outer boundary neighbor.

A new compiled-path integration target ran all 35 frozen fixtures through the
real `scrub inspect ... --json` executable and report parser. Exact mechanism,
version, status, evidence names/values, counts, retained JSON, truncation,
offsets, abbreviations, full artifact length/SHA-256, finding order, and input
non-modification checks passed. This included the six required negative text
classes; ZWJ separation; all 12 identities; structure-shaped input ordering;
complete DICP overlap; exact 256/257 retention; the valid RLO at byte/scalar
offset 65,535; and the 131,074-byte malformed boundary artifact whose third read
was fully hashed and counted while both property findings discarded prefix
evidence and finalized `INVALID`.

Eight real CLI executions over the complete 12-control artifact produced
byte-identical JSON stdout and stderr. The report model retained canonical
finding and evidence ordering, and Bidi locations remained in original input
order. Human-output tests confirmed canonical code point, abbreviation, and
numeric offsets without rendering the raw control or using security/intent
classifications. The top-level limitation now truthfully identifies both
Unicode 17.0.0 observations as evaluated and preserves the remaining excluded
mechanisms.

All requested focused targets passed: 12 binary/unit tests, 5 compiled Bidi
production tests, 11 Bidi fixture-oracle tests, 5 DICP production tests, 5 DICP
fixture tests, and 9 CLI tests. `cargo fmt --check`, Clippy for all workspace
targets with warnings denied, all 55 workspace tests, `git diff --check`, and
`just check` passed. Final scope inspection found no Cargo manifest/lock, report
schema, `CONFORMANCE.md`, C2PA/watermark, unrelated seed-document, index, or Git
history mutation. Pre-existing unrelated untracked documentation remains
untouched. No contradiction, blocker, or ambiguity was found for independent
review.
