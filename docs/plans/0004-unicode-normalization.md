# Milestone 4A: Unicode 17.0.0 normalization source authority and reporting contract

## Goal

Pin the Unicode 17.0.0 normalization authorities and raw source identities, and
freeze the future NFC/NFKC difference-reporting contract before fixtures or
production implementation exist.

This milestone changes documentation and source provenance only. It does not
make normalization a supported scrub.ts capability.

## Non-goals

- Rust, test, fixture, Cargo, dependency, report-schema, or production
  conformance changes.
- Normalization, rewriting, sanitization, replacement, output-file generation,
  or any modified artifact.
- A production architecture, read-count or buffering policy, or whole-file
  representation.
- Approval of the `unicode-normalization` crate or any other dependency.
- Public NFD or NFKD findings in v0.1.
- UTS #39 confusable skeletons, script or identifier checks, spoof detection,
  or another Unicode security mechanism.
- C2PA parsing, embedding, hashing, or validation.
- Security, provenance, watermark, AI-authorship, or Claude-authorship
  inference from normalization.
- A generic scanner, registry, plugin, manager, factory, or Unicode framework.
- A production-support claim in `README.md` or `CONFORMANCE.md`.

## Sources / authority

- Unicode Standard Annex #15, Unicode Normalization Forms, Unicode 17.0.0,
  Revision 57, dated 2025-07-30, is the normative semantic and conformance
  authority.
- Unicode Standard Annex #44 and the Unicode 17.0.0 UCD govern data-file and
  property interpretation.
- Unicode 17.0.0 `NormalizationTest.txt` is the primary executable conformance
  oracle under UAX15-C3.
- Pinned Unicode 17.0.0 `DerivedAge.txt` supplies authoritative Age data and
  defines assigned versus Unassigned code points for the test-header identity
  requirement.
- Pinned Unicode 17.0.0 `DerivedNormalizationProps.txt`,
  `NormalizationCorrections.txt`, `UnicodeData.txt`, and
  `CompositionExclusions.txt` supply the required UCD data roles.
- Unicode License V3 governs the local data corpus.
- `docs/SOURCE_AUTHORITY.md`, `docs/specs/report-schema.md`, and the existing
  Unicode contracts govern scrub.ts evidence and invalid-UTF-8 conventions.
- C2PA Technical Specification 2.4 is downstream interoperability authority
  only for its unstructured-text content-binding procedure. It is not Unicode
  normalization authority.

Independent verification on 2026-08-12 established:

- `NormalizationTest.txt`: 2,827,429 bytes; SHA-256
  `5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db`;
- `DerivedAge.txt`: 138,286 bytes; SHA-256
  `f8ecdf768bdc210f201abd271d9bc587825618a86a7046a8146cc816393f1998`;
- `DerivedNormalizationProps.txt`: 1,377,582 bytes; SHA-256
  `71fd6a206a2c0cdd41feb6b7f656aa31091db45e9cedc926985d718397f9e488`;
- `NormalizationCorrections.txt`: 2,214 bytes; SHA-256
  `32cdfbfd01844f1fcd0b3c9da53a1f9cba7d3eeaba278f8d1330fdafd57e85c0`;
- `UnicodeData.txt`: 2,198,209 bytes; SHA-256
  `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c`;
- `CompositionExclusions.txt`: 9,007 bytes; SHA-256
  `2f239196ef3b5b61db5cc476e9bd80f534d15aa1b74e1be1dea5d042a344c85f`;
- `UNICODE-LICENSE.txt`: 1,995 bytes; SHA-256
  `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

All required files are locally present and their raw identities match the
independent official-UCD values supplied for this milestone.

## Current state

Production currently performs one incremental UTF-8 decode pass and emits
Unicode 17.0.0 DICP and Bidi_Control property findings. The decoder emits
validated scalars with original byte and scalar offsets while the outer loop
hashes and counts the same 65,536-byte read slices. Invalid UTF-8 discards
valid-prefix property evidence but complete artifact hashing continues.

Normalization is listed as unevaluated. There is no normalization source
ledger entry, reporting contract, fixture oracle, scanner, dependency, or
conformance claim. The required raw Unicode files exist only in the ignored
research library.

## Design

Add `docs/specs/unicode-normalization.md` and bind its sources in
`research/sources.yaml`. Keep the proposed public mechanisms:

- `unicode.normalization.nfc_difference`;
- `unicode.normalization.nfkc_difference`.

Both use version `17.0.0`. For complete valid UTF-8, each finding compares the
complete original scalar sequence with the complete result of its named form.
Difference is `PRESENT`; identity is `ABSENT`. Malformed or incomplete UTF-8 is
`INVALID`, with only the existing complete-artifact UTF-8 validation evidence
and no normalized-prefix identity.

NFC and NFKC remain distinct because they answer canonical and compatibility
normalization questions, respectively. NFD and NFKD remain required for
conformance verification but are not public v0.1 findings because ordinary
precomposed text would frequently differ under decomposition without adding
proportional forensic value.

For valid `PRESENT`, freeze complete normalized SHA-256, normalized UTF-8 byte
length, normalized scalar count, and one bounded first positional divergence.
The first divergence retains at most eight canonical `U+XXXX` scalars from
each side and an original byte comparison position. Valid `ABSENT` has no
redundant mechanism evidence. `INVALID` has no normalization evidence.

The contract prohibits chunk-local normalization, silent Stream-Safe Text,
CGJ insertion, mutation, and constant-memory promises for arbitrary valid
input. Architecture and dependency decisions remain open until after the
official oracle and curated fixture contract are frozen.

## Milestone 4B oracle and fixture requirements

Milestone 4B must create no production scanner. It must first build an
independent test oracle from the complete pinned `NormalizationTest.txt` file.
It must check in an unconditional verbatim copy at
`crates/scrub/tests/fixtures/NormalizationTest-17.0.0.txt`, covered by the
existing fixture-area Unicode License V3 notice and retaining the exact source
identity and digest. The required suite must run from a clean clone without the
ignored research library; that local copy may only add source-parity checking.
4B must not add a compression or decompression dependency merely for this
oracle.

The parser must recognize the five semantic columns
`source ; NFC ; NFD ; NFKC ; NFKD`, exercise all 20,034 literal data records,
and verify every header-defined transformation relationship for all four forms.
Literal expected sequences must come only from the official columns. No
normalization implementation, production helper, or second library may
generate, rewrite, fill, or replace expected sequences. Oracle helpers and
production normalization helpers must not import one another. Curated expected
public findings must be frozen independently, not generated by executing
production code. The official suite may not be sampled, skipped, or made
optional when the ignored corpus is absent.

For the header's assigned-code-point identity requirement, 4B must derive the
complete assigned set from pinned Unicode 17.0.0 `DerivedAge.txt`, subtract the
set of code points appearing as Part 1 source values, and distinguish the full
normative complement from the scrub-supported valid-UTF-8 domain. Independent
4A parsing found 299,448 assigned code points, 17,086 unique Part 1 source code
points, and a 282,362-code-point complement. The complement contains 2,048
surrogates, which cannot occur in valid UTF-8, and 280,314 assigned Unicode
scalar values, every one of which 4B must exercise through all four forms.
Scalar-only execution must not be presented as unqualified conformance over
surrogate-containing code-point sequences.

Valid UTF-8 containing an unassigned scalar remains valid artifact input; it is
not `INVALID` merely because the scalar is Unassigned in Unicode 17.0.0. Such
code points use the pinned version's applicable default normalization-property
behavior. Before freezing exhaustive unassigned-scalar coverage, 4B must derive
and document the exact obligation and expected behavior from the pinned UAX
#15/UAX #44 and UCD defaults. It must not invent an assigned-character-only
runtime restriction or expand into an NPSS implementation.

The official conformance suite and the curated artifact corpus serve different
purposes and both are mandatory.

### Required categories

The curated corpus must include at least:

- empty UTF-8;
- ASCII identity;
- ordinary multilingual text already NFC;
- precomposed versus canonically decomposed accent;
- canonical combining-class reordering;
- blocked composition;
- algorithmic Hangul decomposition and composition;
- compatibility ligature;
- fullwidth compatibility characters;
- superscript and circled compatibility examples;
- input where NFC is unchanged but NFKC changes;
- input where NFC changes;
- divergence after a multibyte BMP scalar prefix;
- divergence after a supplementary-plane scalar prefix;
- exactly eight scalars retained in a `first_difference` window;
- a window with at least nine scalars proving `truncated=true`;
- compatibility decomposition followed by composition that contracts the
  sequence;
- variation selectors and emoji for which normalization must not create a fake
  forensic claim;
- a normalization-sensitive sequence crossing an arbitrary short chunk
  boundary;
- a valid multibyte UTF-8 scalar split across the real 65,536-byte reader
  boundary;
- a separate normalization-sensitive sequence split at the real 65,536-byte
  boundary between complete UTF-8 scalars;
- back-to-back normalization-sensitive sequences;
- a very long combining/non-starter sequence that exposes fixed-buffer
  assumptions;
- the malformed UTF-8 forms already established by the Unicode scanner
  contract;
- a valid normalization-sensitive prefix followed by malformed UTF-8;
- malformed UTF-8 spanning the real read boundary;
- late malformed UTF-8 after multiple successful reads;
- trailing bytes after that malformed sequence, with the complete trailing data
  included in artifact byte length and SHA-256;
- complete artifact SHA-256 and byte-length preservation;
- non-modification of the original input;
- deterministic repetition of every corresponding public evidence value.

The curated expectations must freeze both public findings separately,
including cases where both are absent, both are present, and NFC is absent
while NFKC is present. Every `PRESENT` case must freeze all four evidence
entries and the exact compact `first_difference` JSON. Every `ABSENT` case must
freeze an empty normalization evidence list. Every `INVALID` case must freeze
the existing UTF-8 validation evidence and absence of normalized-prefix data.

## Acceptance criteria

- Every required local source exists and independently matches its pinned byte
  length and SHA-256.
- The source ledger records UAX #15 Revision 57, UAX #44's interpretation role,
  all six raw Unicode 17.0.0 data identities, and the existing Unicode license.
- The reporting contract explicitly fixes sequence-level and complete-input
  semantics, the two mechanism IDs/version, all statuses, and exact evidence.
- The contract explains why arbitrary chunks cannot be normalized
  independently and why `Quick_Check=MAYBE` is not a final result.
- The contract records the NFKC compatibility warning and every required
  neutral non-claim.
- NFC/NFKC are public findings; NFD/NFKD remain conformance-only in v0.1.
- `first_difference` has one deterministic bounded representation and is not
  described as a cause or suspicious location.
- `ABSENT` evidence is explicitly empty and `INVALID` retains no normalized
  prefix evidence.
- The contract prohibits mutation and silent Stream-Safe Text, makes no
  arbitrary-valid-input constant-memory promise, and defers architecture and
  dependency selection.
- Milestone 4B's complete official oracle and every required curated category
  are frozen without creating fixtures.
- UTS #39 and C2PA remain downstream, separate mechanisms.
- Only the two milestone documents and `research/sources.yaml` change. No Rust,
  test, fixture, Cargo, report schema, `CONFORMANCE.md`, README capability
  claim, unrelated seed document, Git index, or history changes.

## Implementation steps

1. Verify presence, byte size, SHA-256, identifying headers/dates, and license
   identity for the complete local Unicode normalization corpus.
2. Review UAX #15 Revision 57, UAX #44/UCD conventions, the official test-file
   header, existing report ontology, current Unicode stream, and prior fixture
   conventions.
3. Freeze source hierarchy, status semantics, evidence representation, neutral
   interpretation, sequence/chunk boundary, and Stream-Safe constraints in the
   normalization specification.
4. Freeze the Milestone 4B official-oracle and curated-fixture requirements,
   while leaving production architecture and dependency selection open.
5. Update `research/sources.yaml` narrowly, validate it, run the repository
   quality gate and Git checks, and inspect the complete changed-path scope.

## Validation

- Local PowerShell byte-length and SHA-256 recomputation for all six raw UCD
  files and the existing Unicode License V3 text.
- Header/date inspection for every source that contains an identifying header.
- Local parsing of `research/sources.yaml` using the repository's established
  available method.
- `git diff --check`
- `just check`
- `git diff --stat`
- `git diff`
- `git status --short`
- `git diff --cached --name-only`
- Changed-path assertions for no Rust, tests, fixtures, Cargo, report schema,
  `CONFORMANCE.md`, README capability, raw research source, or unrelated seed
  changes.

## Risks / open questions

- A later library can claim Unicode 17.0.0 support yet still differ in
  conformance, streaming behavior, or embedded data revision. Exact dependency
  review and official-corpus testing remain mandatory.
- Normalization is not closed under arbitrary concatenation. Any design that
  treats decoder pushes as independent strings will be incorrect.
- Valid input can contain extremely long non-starter sequences. A fixed
  unresolved buffer would either fail conformant input or silently change it;
  neither behavior is approved here.
- The existing decoder is push-based while plausible normalization libraries
  are iterator-based. The smallest conformant bridge remains undecided.
- Production and oracle paths must remain independent enough that one defect
  cannot generate both the implementation and its expected result.
- Resource and arithmetic-overflow behavior for pathological valid input must
  be specified with the eventual architecture. Any such failure that prevents
  a complete normalization decision must remain outside `PRESENT`, `ABSENT`,
  and `INVALID`.
- No direct contradiction currently requires redesign of `first_difference`
  `at_end` or `truncated`; the absence of a proper-prefix pair in reviewed
  official transformations remains a 4B test-design consideration, not a
  schema change in 4A.

## Outcome

Milestone 4A established the Unicode 17.0.0 normalization source and reporting
contract without production support. An initial independent review returned
PASS WITH REQUIRED FIXES. This correction strengthened assignment authority,
clean-clone oracle availability and implementation independence,
resource-failure classification boundaries, and required 4B adversarial
coverage. `DerivedAge.txt` is now pinned as the assignment authority for the
official test header.

The specification freezes two separate public mechanism IDs, complete-input
`PRESENT`/`ABSENT`/`INVALID` semantics,
the valid and invalid evidence contract, an eight-scalar first-divergence
window, neutral interpretation, no-mutation behavior, chunk/concatenation
constraints, and the pathological-buffering boundary. NFD and NFKD remain
conformance-only in v0.1. Architecture and dependency selection remain
deliberately open.

All six local raw Unicode files matched their pinned byte lengths and SHA-256
values, and the existing Unicode License V3 identity also matched. The source
ledger parsed with local PyYAML and had unique source IDs. `git diff --check`
and the separate new-document trailing-whitespace scan passed. `just check`
passed formatting, Clippy with warnings denied, and all 55 existing tests. The
raw research corpus remained ignored.

No production normalization support, dependency, Rust code, fixture, Cargo,
report-schema, README claim, or `CONFORMANCE.md` support claim was added. The
raw research corpus remained ignored and read only. A fresh independent
re-review is still required; this outcome does not claim that it has passed.

Final Git scope inspection found no Rust, test, fixture, Cargo, report-schema,
`CONFORMANCE.md`, README capability, raw research source, index, or history
change. The only milestone files are this plan,
`docs/specs/unicode-normalization.md`, and `research/sources.yaml`;
pre-existing unrelated untracked seed documents remain untouched.
