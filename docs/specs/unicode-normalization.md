# Unicode 17.0.0 normalization-difference reporting

## Scope and support boundary

This contract defines what future deterministic inspection may report about a
complete byte artifact interpreted as UTF-8 under Unicode 17.0.0 normalization.
It does not establish production support. Production support remains absent
until the official conformance oracle, curated fixtures, implementation, and
`CONFORMANCE.md` entry are complete.

The two public mechanism identifiers are:

- `unicode.normalization.nfc_difference`;
- `unicode.normalization.nfkc_difference`.

Both use mechanism version `17.0.0`. They report a neutral relation between the
original valid Unicode scalar sequence and one named normalization form. They
are not character-property membership scanners and do not reuse the DICP or
Bidi_Control `locations` evidence model.

## Source authority

The authority hierarchy for this contract is:

1. Unicode Standard Annex #15, Unicode Normalization Forms, Unicode 17.0.0,
   Revision 57, dated 2025-07-30, governs canonical and compatibility
   equivalence, NFC/NFD/NFKC/NFKD, conformance, normalization stability,
   `Quick_Check`, concatenation and boundaries, and buffering constraints.
2. Unicode Standard Annex #44, Unicode Character Database, and the Unicode
   17.0.0 UCD govern interpretation of the normalization data files.
3. Unicode 17.0.0 `NormalizationTest.txt` is the primary executable conformance
   oracle required by UAX #15 conformance clause UAX15-C3.
4. Unicode 17.0.0 `DerivedAge.txt` is the assignment authority for the
   `NormalizationTest.txt` header's assigned-code-point identity requirement.
5. The pinned Unicode 17.0.0 UCD files supply decomposition mappings, canonical
   combining classes, normalization properties and corrections, and composition
   exclusions as specified by UAX #15 and UAX #44.

The ignored local source corpus was independently verified on 2026-08-12:

| Source | Bytes | SHA-256 |
|---|---:|---|
| `NormalizationTest.txt` | 2,827,429 | `5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db` |
| `DerivedAge.txt` | 138,286 | `f8ecdf768bdc210f201abd271d9bc587825618a86a7046a8146cc816393f1998` |
| `DerivedNormalizationProps.txt` | 1,377,582 | `71fd6a206a2c0cdd41feb6b7f656aa31091db45e9cedc926985d718397f9e488` |
| `NormalizationCorrections.txt` | 2,214 | `32cdfbfd01844f1fcd0b3c9da53a1f9cba7d3eeaba278f8d1330fdafd57e85c0` |
| `UnicodeData.txt` | 2,198,209 | `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c` |
| `CompositionExclusions.txt` | 9,007 | `2f239196ef3b5b61db5cc476e9bd80f534d15aa1b74e1be1dea5d042a344c85f` |

The first four files with identifying headers report, respectively,
`NormalizationTest-17.0.0.txt` dated `2025-06-30, 06:16:16 GMT`,
`DerivedAge-17.0.0.txt` dated `2025-07-30, 23:54:38 GMT`,
`DerivedNormalizationProps-17.0.0.txt` dated
`2025-01-27, 18:09:14 GMT`, and
`NormalizationCorrections-17.0.0.txt` dated `2025-08-01`.
`CompositionExclusions.txt` reports `CompositionExclusions-17.0.0.txt` dated
`2025-08-01`. `UnicodeData.txt` has no identifying header; its raw identity is
therefore bound by path, byte length, and digest.

The files are distributed under Unicode License V3. The local license text is
`research/library/unicode/17.0.0/UNICODE-LICENSE.txt`: 1,995 bytes, SHA-256
`e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.
Raw Unicode source files remain ignored research inputs and are not production
fixtures.

C2PA 2.4 is a downstream interoperability source only. It does not define
Unicode normalization semantics and cannot override UAX #15.

## Sequence semantics

Normalization is defined over a Unicode string or sequence. It is not an
independent property of each scalar value:

- NFD is canonical decomposition;
- NFC is canonical decomposition followed by canonical composition;
- NFKD is compatibility decomposition;
- NFKC is compatibility decomposition followed by canonical composition.

For any form NFx, UAX #15 defines `isNFx(s)` as true if and only if `toNFx(s)`
is identical to `s`. For a valid UTF-8 artifact, scrub.ts evaluates this by
comparing the UTF-8 encoding of the complete normalized scalar sequence with
the complete original bytes. UTF-8 has one well-formed encoding for a scalar
sequence, so byte-for-byte identity and scalar-sequence identity agree here.

Composition, decomposition, expansion, contraction, and canonical reordering
can involve adjacent scalars. A single scalar cannot in general be identified
as an independent location or cause of a normalization difference.

None of the four normalization forms is closed under arbitrary string
concatenation. Two independently normalized strings can produce a non-normalized
concatenation, including in NFD because canonical ordering can cross the join.
Consequently, normalizing each file read, decoder callback batch, or other
arbitrary chunk and concatenating the results is not conformant. A finding must
represent the complete Unicode scalar sequence even if a later implementation
computes it incrementally.

`Quick_Check=MAYBE` is contextual. It can trigger more work but cannot itself
be reported as the final normalized/not-normalized classification. A
`PRESENT` or `ABSENT` result must be equivalent to applying the named form and
testing complete-sequence identity.

## Public findings and status semantics

The findings are separately evaluated because NFC preserves canonical
equivalence while NFKC additionally folds compatibility distinctions. One
artifact can therefore be unchanged by NFC and changed by NFKC. NFKC-normalized
text is necessarily NFC-normalized, so NFC `PRESENT` with NFKC `ABSENT` is not
an expected combination.

| Complete artifact result | Status |
|---|---|
| Valid UTF-8 and the named Unicode 17.0.0 normalized UTF-8 sequence differs byte-for-byte from the original | `PRESENT` |
| Valid UTF-8 and the named Unicode 17.0.0 normalized UTF-8 sequence is byte-for-byte identical to the original | `ABSENT` |
| Malformed or incomplete UTF-8 anywhere in the artifact | `INVALID` |

There is no normal `UNKNOWN` path for a supported deterministic normalization
finding. `PRESENT` or `ABSENT` may be emitted only after successful comparison
of the complete valid-UTF-8 artifact with its complete named normalization.
`INVALID` is reserved for malformed or incomplete UTF-8 artifact input. An
internal execution, resource, or arithmetic failure that prevents the decision
from completing must not be converted into `PRESENT`, `ABSENT`, or `INVALID`.
The existing interface treats observer/internal failure as command failure;
detailed future error types, wording, allocation and overflow policy, buffering,
and CLI/internal plumbing remain implementation decisions.

For either mechanism, `PRESENT` means only:

> Applying the named Unicode 17.0.0 normalization form produces a Unicode
> scalar sequence whose UTF-8 encoding is not byte-for-byte identical to the
> original valid UTF-8 artifact.

`PRESENT` is not a security, provenance, authorship, or intent verdict.
In particular, an NFKC difference must never be described as malformed text,
invalid Unicode, malicious, suspicious, unsafe, spoofing, watermarking, AI
generation, or Claude generation. UAX #15 warns that NFKC and NFKD must not be
blindly applied to arbitrary text because compatibility normalization can erase
formatting distinctions and distinctions important to semantics.

## NFC and NFKC public scope

NFC and NFKC remain separate public findings in v0.1:

- NFC difference records canonical representation identity and supports later
  standards interoperability where NFC-normalized bytes matter.
- NFKC difference records the distinct compatibility-normalization relation.
  Keeping it separate prevents compatibility folding from being confused with
  canonical normalization or interpreted as damage or invalidity.

NFD and NFKD are required in source and conformance verification but are not
separate public findings in v0.1. Public decomposition-form findings would make
ordinary precomposed multilingual text positive without proportional forensic
value. This is a scrub.ts reporting-scope decision, not a claim that NFD or
NFKD is less conformant or less normative. UAX #15 defines all four forms and
does not require an inspection product to publish all four as findings. The
primary sources do not contradict this scope choice.

## Evidence contract

The mechanism identifier supplies the named form, so a separate form evidence
entry would be redundant. Evidence values use the existing schema 0.1
string-valued representation.

### Valid `PRESENT`

A `PRESENT` finding contains exactly these normalization evidence entries:

- `normalized_sha256`: lowercase 64-digit SHA-256 of the UTF-8 encoding of the
  complete normalized Unicode scalar sequence;
- `normalized_byte_length`: the complete normalized UTF-8 byte length as an
  unsigned base-10 integer string with no leading zeroes except `0`;
- `normalized_scalar_count`: the complete normalized Unicode scalar count in
  the same integer representation;
- `first_difference`: one compact JSON object, encoded as a string evidence
  value, with the deterministic representation below.

The existing canonical report ordering sorts these evidence names as
`first_difference`, `normalized_byte_length`, `normalized_scalar_count`, then
`normalized_sha256`.

scrub.ts does not embed or emit the complete normalized text. A `PRESENT`
finding intentionally exposes its normalized digest, lengths/count, and bounded
first-difference scalar window. The digest is an equality oracle, not a
confidentiality mechanism. This evidence does not replace the top-level
artifact SHA-256 and byte length, which identify the untouched input.

### `first_difference`

Let `i` be the lowest zero-based scalar index at which the complete original
and normalized sequences have different scalar values, or at which exactly one
sequence has reached its end. `first_difference.scalar_index` is `i`.

`original_byte_offset` is the byte offset of original scalar `i`. If the
original sequence reaches its end at `i`, it is the original artifact byte
length, which identifies the end-of-input comparison position.

Each side retains at most eight scalars beginning at `i`. Eight is a fixed
diagnostic bound: it exposes a small immediate composition, decomposition,
expansion, contraction, or reordering window while keeping evidence size
independent of input and normalization expansion. It is not intended to encode
a complete edit or explanation; the complete normalized identity is represented
by the digest and lengths.

The compact JSON has no insignificant whitespace and uses this exact key order:

```json
{"scalar_index":0,"original_byte_offset":0,"original":{"at_end":false,"scalars":["U+0065","U+0301"],"truncated":false},"normalized":{"at_end":false,"scalars":["U+00E9"],"truncated":false}}
```

The representation rules are:

- JSON integers are unsigned base-10 numbers.
- Every scalar is uppercase hexadecimal in `U+XXXX` notation, with at least
  four digits and no unnecessary leading zeroes above `U+FFFF`.
- `at_end` is true if and only if that side has no scalar at `i`; in that case
  `scalars` is empty and `truncated` is false.
- Otherwise, `scalars` contains the next one through eight scalars from that
  side. `truncated` is true if and only if additional scalars remain after the
  eighth retained scalar.
- No raw scalar, invisible character, or control character is emitted.

This is the first observable positional divergence between two scalar
sequences. It is not a cause, suspicious location, semantic alignment, edit
script, grapheme boundary, display position, or claim that the retained scalars
are independently normalizable. Expansion or contraction can shift all later
positional comparisons.

### Valid `ABSENT`

An `ABSENT` finding has no normalization evidence entries. The status already
asserts that the normalized UTF-8 bytes equal the original bytes, so repeating
the artifact hash, byte length, or scalar count under normalized names would be
redundant. The top-level artifact SHA-256 and byte length remain the auditable
identity. Fixture work must not invent additional `ABSENT` evidence.

### `INVALID`

An `INVALID` finding contains exactly the existing UTF-8 validation evidence:

```text
utf8_validation=failed: the complete artifact is malformed or incomplete UTF-8
```

It contains no `normalized_sha256`, `normalized_byte_length`,
`normalized_scalar_count`, or `first_difference` evidence from a valid prefix.
The top-level artifact SHA-256 and byte length still cover the complete original
artifact. A finding limitation may explain that normalization evidence is
unavailable because the complete artifact is not valid UTF-8.

## Inspection-only and neutral reporting

The scanner never normalizes, rewrites, sanitizes, replaces, fixes, or emits a
modified artifact. It does not generate an output file or claim removal. The
normalized sequence exists only as an internal comparison result and bounded
identity evidence. Original artifact hashing and Unicode inspection must derive
from the same bytes, and the report always identifies the untouched input.

Normalization differences do not establish confusability, spoofing, security
risk, watermark presence, AI authorship, Claude authorship, or provenance.
Variation selectors and emoji sequences must not acquire such claims merely
because they are Unicode sequences evaluated by this mechanism.

## Unicode versioning and stability

Mechanism version `17.0.0` pins the UAX #15 and UCD behavior used for the
decision and normalized identity. UAX #15 normalization stability protects
normalized assigned text across later versions, subject to its documented
versioning rules; it does not justify silently substituting host-library data
or a later Unicode version. Reports remain explicitly versioned so results can
be reproduced against the pinned sources, including for code points whose
assignment status can differ across Unicode versions.

## Streaming and buffering boundary

Later production code must be correct for arbitrary `Read::read` partitioning.
A normalization-relevant sequence can cross a short test partition, a decoder
callback boundary, or the real 65,536-byte inspection boundary. Chunk-local
normalization is prohibited by the complete-sequence and concatenation
semantics.

UAX #15 permits valid strings with extremely long non-starter sequences. Such
input can require a large unresolved buffer before canonical ordering can be
serialized. The Stream-Safe Text Process obtains a fixed bound by inserting
U+034F COMBINING GRAPHEME JOINER into long non-starter sequences; UAX #15 notes
that when this changes input, the result is not canonically equivalent to the
original. scrub.ts must not silently impose Stream-Safe Text, insert CGJ, or
otherwise change the interpreted sequence.

The contract therefore promises no constant-memory bound for arbitrary valid
Unicode normalization. A later implementation may use a growable unresolved
normalization segment or another conformant incremental design. The finding
must be based on the same complete artifact bytes whose identity is reported,
and arbitrary read partitioning must not change the result. Exact read count,
buffering, architecture, and resource policy are deferred until after the
oracle is frozen.

## Official conformance oracle

Unicode 17.0.0 `NormalizationTest.txt` has five semicolon-separated semantic
columns:

```text
source ; NFC ; NFD ; NFKC ; NFKD
```

For columns `c1` through `c5`, its header requires:

```text
NFC:  c2 == toNFC(c1) == toNFC(c2) == toNFC(c3)
      c4 == toNFC(c4) == toNFC(c5)
NFD:  c3 == toNFD(c1) == toNFD(c2) == toNFD(c3)
      c5 == toNFD(c4) == toNFD(c5)
NFKC: c4 == toNFKC(c1) == toNFKC(c2) == toNFKC(c3)
                         == toNFKC(c4) == toNFKC(c5)
NFKD: c5 == toNFKD(c1) == toNFKD(c2) == toNFKD(c3)
                         == toNFKD(c4) == toNFKD(c5)
```

The header also requires identity under all four forms for every code point
assigned in Unicode 17.0.0 that is not specifically listed as a Part 1 source.
For this contract, "assigned" is defined by the pinned Unicode 17.0.0 Age data
in `DerivedAge.txt`, not by mere record presence in `UnicodeData.txt`.
Independent parsing yields 299,448 assigned code points and 17,086 unique Part
1 source code points. Their complete set difference has 282,362 code points:
2,048 surrogates and 280,314 Unicode scalar values.

Milestone 4B must:

1. exercise every one of the 20,034 literal data records and every
   header-defined transformation relationship for all four forms;
2. derive the complete assigned-code-point set from the pinned
   `DerivedAge.txt` and subtract the code points appearing as Part 1 source
   values;
3. preserve the distinction between the full normative assigned-code-point
   complement and scrub.ts's executable valid-UTF-8 domain;
4. exhaustively exercise every assigned Unicode scalar value in that complement
   through NFC, NFD, NFKC, and NFKD; and
5. record that the 2,048 assigned surrogate code points are in the broader
   complement but cannot occur in a well-formed UTF-8 scrub artifact.

Scalar-only execution does not by itself establish an unqualified UAX #15
conformance claim over surrogate-containing code-point sequences.

Unassigned code points are not thereby invalid. For the pinned Unicode version,
they use the applicable default normalization-property behavior. A valid UTF-8
artifact containing an unassigned Unicode scalar value remains valid artifact
input. Milestone 4B must derive and document its exhaustive unassigned-scalar
test obligation and expected behavior from the pinned UAX #15/UAX #44 and UCD
defaults; this contract does not invent an assigned-characters-only product
restriction or expand into a Normalization Process for Stabilized Strings.

Milestone 4B must add
`crates/scrub/tests/fixtures/NormalizationTest-17.0.0.txt` as an unconditional,
verbatim copy of the complete pinned official file. The existing fixture-area
Unicode License V3 notice supplies the required license attribution, and the
test contract must retain the exact official source identity and digest above.
The conformance suite must run from a clean clone; absence of the ignored
research-library copy must not skip or make it optional. That ignored copy may
remain an independent source-parity input.

Literal expected normalized sequences come only from the official columns. No
normalization implementation or library may generate, rewrite, fill, or replace
them, including a second normalization library. The oracle must not import
production normalization helpers, and production code must not import oracle
helpers. Curated expected public findings must be frozen independently rather
than generated by executing production code. Milestone 4B must not add a
compression or decompression dependency merely for this oracle. A small set of
hand-selected examples or a sample of the 20,034 records cannot substitute for
the complete official suite.

The curated artifact corpus has a different purpose and is also required. It
must freeze exact public statuses and evidence, hostile-input behavior,
artifact identity, partition invariance, and non-modification. Its required
categories are recorded in `docs/plans/0004-unicode-normalization.md`.

## Downstream boundaries

UTS #39 confusable processing is separate future work. Normalization is a
prerequisite used by some Unicode security mechanisms, but a normalization
difference is not confusability. This mechanism does not implement skeletons,
script checks, identifier restrictions, spoof detection, or any other UTS #39
security mechanism.

C2PA 2.4 unstructured-text content binding is a concrete downstream reason NFC
identity matters: its text data-hash validation removes the specified wrapper
bytes, normalizes the remaining text to NFC, encodes it as UTF-8, and hashes
those bytes. Its exclusion handling defines offsets against NFC-normalized
UTF-8 bytes. This note does not implement C2PA, does not make C2PA an authority
for normalization, and does not imply that every variation-selector sequence
is C2PA data.

## Deliberately deferred implementation decisions

Milestone 4A selects no production normalization architecture or dependency.
A later implementation milestone must decide and record:

- the conformant incremental segmentation/buffering strategy and integration
  with the existing push-based UTF-8 decoder;
- how hashing, NFC, NFKC, scalar comparison, and first-difference evidence
  operate over the same complete artifact bytes;
- resource and overflow error types, wording, policies, and plumbing for
  pathological but valid input, subject to the status invariant above;
- whether the `unicode-normalization` crate is suitable, including exact
  release/revision, Unicode version, license, maintenance, conformance, and
  iterator/streaming behavior;
- how production testing remains genuinely independent from its oracle and
  does not use the same normalization implementation to create expected data.

No dependency is approved by this contract.
