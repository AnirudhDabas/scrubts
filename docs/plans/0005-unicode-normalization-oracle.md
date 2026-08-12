# Milestone 4B-1: Unicode 17.0.0 normalization conformance oracle

## Goal

Commit the complete official Unicode 17.0.0 normalization and assignment data
needed to judge a future production normalizer, and provide a test-only parser
that exposes every literal conformance expectation without normalizing text.

This milestone builds the judge before selecting or implementing the contestant.
It does not establish production normalization support.

## Non-goals

- Production normalization or changes under `crates/scrub/src/`.
- Selection or addition of a normalizer dependency.
- Public NFC or NFKC findings, report changes, or conformance claims.
- The curated scrub adversarial artifact corpus reserved for Milestone 4B-2.
- A `first_difference` fixture matrix.
- A 65,536-byte normalization boundary corpus.
- C2PA work.
- WaterLARP work.
- A generic Unicode parser, oracle framework, registry, plugin, manager, or
  production data loader.

## Sources / authority

- Unicode Standard Annex #15, Unicode Normalization Forms, Unicode 17.0.0,
  Revision 57, dated 2025-07-30, is the semantic and conformance authority.
- UAX #15 conformance clause UAX15-C3 requires the results in the official
  `NormalizationTest.txt` conformance data.
- Unicode Standard Annex #44, Unicode Character Database, Unicode 17.0.0,
  Revision 36, dated 2025-08-27, governs UCD defaults and file interpretation.
- Unicode 17.0.0 `NormalizationTest.txt` supplies every literal five-column
  row and the header relationships used as expected transformations.
- Unicode 17.0.0 `DerivedAge.txt` supplies assignment. Its `@missing` rule makes
  every unlisted code point Unassigned.
- Unicode 17.0.0 `DerivedNormalizationProps.txt` supplies the four normalization
  Quick_Check `@missing` defaults.
- Unicode License V3 permits the two official data files to be committed under
  the existing fixture-area license notice.

The local primary-source identities verified before editing were:

| Source | Bytes | SHA-256 |
|---|---:|---|
| `UAX15-Rev57.html` | 141,627 | `c0c05f91e1c4f9be3d987e27d76cf254b30003b6be41eb94977cc3fe148d4c4e` |
| `UAX44-Rev36.html` | 313,639 | `a97ed3f9dbee8e9a917106a3dc49a0fdeead42e2be06ea6f5cc31117eda3da33` |
| `NormalizationTest.txt` | 2,827,429 | `5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db` |
| `DerivedAge.txt` | 138,286 | `f8ecdf768bdc210f201abd271d9bc587825618a86a7046a8146cc816393f1998` |
| `DerivedNormalizationProps.txt` | 1,377,582 | `71fd6a206a2c0cdd41feb6b7f656aa31091db45e9cedc926985d718397f9e488` |
| `NormalizationCorrections.txt` | 2,214 | `32cdfbfd01844f1fcd0b3c9da53a1f9cba7d3eeaba278f8d1330fdafd57e85c0` |
| `UnicodeData.txt` | 2,198,209 | `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c` |
| `CompositionExclusions.txt` | 9,007 | `2f239196ef3b5b61db5cc476e9bd80f534d15aa1b74e1be1dea5d042a344c85f` |
| `UNICODE-LICENSE.txt` | 1,995 | `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96` |

## Current state

Milestone 4A pinned the authority and future reporting contract. No official
normalization fixture was committed, and no parser, conformance oracle,
normalization implementation, or normalization dependency existed. The two
required official data files were available only in the ignored research
library, so a clean clone could not run a normalization conformance suite.

The checkout uses `core.autocrlf=true`, and the repository had no
`.gitattributes`. Git's default text heuristics could therefore change official
line endings during add or checkout and invalidate exact-byte fixture identity.

## Design

### Exact committed sources

Commit byte-for-byte copies of the pinned official `NormalizationTest.txt` and
`DerivedAge.txt` files under descriptive versioned fixture names. Mandatory
tests use `include_bytes!`, preserving the distinction between bytes and parsed
text and making the suite independent of the ignored research library.

Add exactly two root `.gitattributes` rules, one for each official fixture,
with `-text`. These path-scoped rules prevent Git content filtering on add and
checkout without changing line-ending behavior elsewhere in the repository.
The rule exists only because raw official byte identity is part of the oracle.

### Independent parser and literal row oracle

Keep one support module under `crates/scrub/tests/support/`. It accepts fixture
bytes, requires UTF-8 syntax, recognizes Parts 0 through 5 in order, parses
exactly five nonempty semicolon-delimited code-point sequences plus the required
trailing semicolon, and rejects unknown directives, malformed rows, malformed
hexadecimal tokens, descending ranges, and values above U+10FFFF.

Oracle sequences are `Vec<u32>`, not `String` or `char`, so the official model
can represent Unicode code points independently of the future production UTF-8
scalar representation. The parser neither imports production code nor calls a
normalization implementation.

For each official row, expose the 20 header-defined transformations through a
small iterator over an explicit relationship table:

- NFC maps input columns c1/c2/c3 to literal c2 and c4/c5 to literal c4;
- NFD maps c1/c2/c3 to literal c3 and c4/c5 to literal c5;
- NFKC maps all five inputs to literal c4;
- NFKD maps all five inputs to literal c5.

The 20,034 rows therefore expose 400,680 expected transformations. No expected
sequence is calculated or rewritten.

### Assignment and identity domains

Parse the actual singleton/range syntax in committed `DerivedAge.txt`, reject
overlap, and expand the 1,815 records into a set of assigned code points. Parse
Part 1 source columns independently and subtract them from assignment. Preserve
the full 282,362-code-point complement and its split into 2,048 surrogates and
280,314 Unicode scalar values.

The header says every assigned code point outside Part 1 is unchanged under all
four forms. The support module exposes that literal identity expectation as an
iterator. It does not execute the future 1,121,256 production comparisons.

### Unassigned scalar obligation

The pinned sources support an exact ordinary-normalization obligation:

1. `DerivedAge.txt` states that unlisted Age values are Unassigned.
2. UAX #44 Section 2.2 and its default-value table give unassigned code points
   their applicable property defaults. `Canonical_Combining_Class` defaults to
   `Not_Reordered` (0), and Section 5.7.3 states that
   `Decomposition_Mapping` defaults to the code point itself.
3. `DerivedNormalizationProps.txt` gives `Yes` as the `@missing` value for
   NFD_QC, NFC_QC, NFKD_QC, and NFKC_QC.
4. UAX #15 Section 12 says NPSS is regular normalization plus an additional
   requirement to abort on code points unassigned in the supported version.
   scrub.ts's contract is ordinary normalization, not NPSS.
5. A Unicode scalar excludes only U+D800..U+DFFF from U+0000..U+10FFFF.
   Assignment status does not make a scalar malformed UTF-8.

Consequently, for Unicode 17.0.0 each single unassigned Unicode scalar is an
identity expectation under NFC, NFD, NFKC, and NFKD. There are 814,664 such
scalars. The domain iterator avoids materializing 3,258,656 duplicate cases.
This single-scalar obligation does not prove behavior for every sequence that
contains an unassigned scalar.

## Anti-tautology boundary

Expected normalized sequences come only from literal official columns and the
official header's column relationships. Assigned and unassigned identity
expectations come only from `DerivedAge.txt` and normative UCD defaults. The
oracle contains no normalization algorithm, calls no normalizer, shares no
normalization helper with production, and introduces no dependency. A future
production implementation can supply only actual outputs; it cannot generate or
alter these expected values.

## Acceptance criteria

- Both committed fixtures are byte-identical to their pinned official sources.
- Fixture length, SHA-256, header/version, and license identity tests are
  unconditional in a clean clone.
- Git attributes make raw and path-aware blob identities equal under the
  maintainer's `core.autocrlf=true` checkout without affecting unrelated paths.
- All 20,034 rows and exact per-Part counts parse from committed bytes.
- Malformed syntax and unknown records fail loudly.
- Literal header expansion yields exactly 400,680 transformations.
- `DerivedAge.txt` yields 1,815 non-overlapping records and 299,448 assigned
  code points.
- Part 1 yields 17,086 unique sources; complements are 282,362 total, 2,048
  surrogates, and 280,314 assigned scalars.
- The independently enumerated scalar domain is 1,112,064 values, with 297,400
  assigned and 814,664 unassigned scalar values.
- Test support exposes assigned-complement and unassigned-scalar four-form
  identity expectations without calling a normalizer.
- Optional parity with ignored local sources runs when they exist but absence
  cannot skip or weaken mandatory committed-fixture tests.
- Offline focused tests and `just check` pass, and the Git index remains empty.

## Implementation steps

1. Complete the source identity, UAX identity, official-record, assignment, and
   scalar-domain pre-edit gate.
2. Commit exact fixture copies and add the two path-scoped `-text` rules.
3. Add the test-only parsers, literal relationship iterator, and identity-domain
   iterators.
4. Add focused tests for raw identity, syntax rejection, structure, counts,
   relationships, assignment arithmetic, and unassigned scalar obligations.
5. Pin the two locally archived UAX HTML identities in the existing ledger
   entries and run all offline validation.

## Validation

- Independent PowerShell SHA-256, byte-size, header, and data-count checks.
- `cargo test --offline -p scrub --test unicode_normalization_oracle`
- `CARGO_NET_OFFLINE=true just check` with `CARGO_TARGET_DIR` outside the
  repository where practical.
- YAML parse and source-ID uniqueness check.
- `git check-attr text eol` for both official fixtures and unrelated paths.
- Raw versus path-aware `git hash-object` checks without `-w`.
- Dependency, production Rust, and documentation claim scans.
- `git diff --check`, complete tracked and untracked file inspection, index
  check, status, and last-log check.

## Risks / open questions

- Future production conformance depends on executing these expectations against
  an independently selected implementation; this milestone freezes only the
  judge and domains.
- The assigned complement includes surrogate code points because DerivedAge
  treats them as assigned. They are deliberately retained in normative domain
  arithmetic but excluded from scrub.ts's valid-UTF-8 scalar execution domain.
- Single-scalar unassigned identity expectations do not establish identity for
  arbitrary surrounding sequences, where canonical ordering or composition may
  involve adjacent assigned characters.

## Outcome

Milestone 4B-1 added verbatim committed-fixture copies of Unicode 17.0.0
`NormalizationTest.txt` and `DerivedAge.txt`. Their source and fixture byte
lengths and SHA-256 values match exactly. A root `.gitattributes` now applies
`-text` to only those two paths. With `core.autocrlf=true`, `git check-attr`
reported `text: unset`; raw and path-aware Git blob identities were equal at
`97b4e4e6202e595f88f07e3a94e8e71b180f66be` for `NormalizationTest` and
`01f13d2c97225a8ff8f2aeea20d4ece6033ea137` for `DerivedAge`. Unrelated paths
retained unspecified text/eol attributes.

One test-only support module now parses all five literal normalization columns,
Parts 0 through 5, and actual DerivedAge range syntax into integer code points.
It exposes the explicit 20-case header relationship iterator, assigned Part 1
complement, Unicode scalar domain, unassigned scalar domain, and four-form
identity expectations. It imports no production module, contains no
normalization algorithm, and uses no normalizer dependency.

Independent parsing and tests reproduced 20,034 rows with Part counts
45/17,086/1,936/194/735/38 and 400,680 literal relationship cases. DerivedAge
produced 1,815 records and 299,448 assigned code points. The Part 1 source set
contains 17,086 values; its assigned complement contains 282,362 code points,
including 2,048 surrogates and 280,314 scalar values. Independent scalar-domain
enumeration produced 1,112,064 scalars, 297,400 assigned scalars, and 814,664
unassigned scalars.

The unassigned identity obligation was resolved from the pinned sources. UAX
#44's default rules, `DerivedAge.txt`, and `DerivedNormalizationProps.txt` give
unassigned values Age=Unassigned, ccc=0, self decomposition mapping, and Yes for
all four normalization Quick_Check properties. UAX #15 Section 12 makes clear
that aborting on unassigned code points is an additional NPSS rule, not ordinary
normalization. The oracle therefore exposes each single unassigned scalar as an
NFC/NFD/NFKC/NFKD identity expectation without claiming arbitrary-sequence
coverage or implementing NPSS.

The focused offline target passed 9 tests. Final `just check`, with
`CARGO_NET_OFFLINE=true` and an external temporary `CARGO_TARGET_DIR`, passed
formatting, Clippy for all targets with warnings denied, and all 64 workspace
tests. The YAML ledger parsed as schema 2 with 43 unique source IDs. Dependency,
production-implementation, and public-support-claim scans found no added
normalizer, production normalization code, or support claim. `git diff --check`
and authored-file trailing-whitespace checks passed. Exact local-source parity
ran because the ignored research files were present; mandatory fixture tests do
not depend on them.

No production Rust, Cargo file, report schema, README, `CONFORMANCE.md`, C2PA,
WaterLARP, or Milestone 4B-2 corpus changed. No file was staged, no commit was
made, no Git history operation ran, and no network access was used. Independent
review returned PASS WITH REQUIRED FIXES. The required CR/CRLF raw-input
rejection and UAX #44 Age-component bounds fixes were applied; independent
re-review remains outstanding. PASS is not claimed.
