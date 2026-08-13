# Milestone 4B-2: production Unicode 17.0.0 normalization

## Goal

Complete launch-quality, inspection-only NFC- and NFKC-difference findings for
whole valid UTF-8 artifacts, and grade the selected production implementation
against the independently committed Unicode 17.0.0 oracle.

## Non-goals

- Rewriting, cleaning, sanitization, Stream-Safe Text, or CGJ insertion.
- Public NFD or NFKD findings.
- Confusable, C2PA, statistical-watermark, Claude-specific, WaterLARP, plugin,
  GUI, web, network, telemetry, database, or daemon work.
- Any Git staging or history operation.

## Sources / authority

- `docs/specs/unicode-normalization.md` freezes public semantics and evidence.
- Unicode 17.0.0 UAX #15 Revision 57 and UAX #44 Revision 36 govern behavior.
- The committed `NormalizationTest-17.0.0.txt` and `DerivedAge-17.0.0.txt`
  fixtures and the test-only parser committed in Milestone 4B-1 are the
  independent conformance judge.
- `unicode-normalization` 0.1.25 is the production contestant, subject to the
  dependency audit and exhaustive grading recorded in this plan's outcome.

## Current state

The CLI has one 65,536-byte file-read loop that hashes and counts all bytes and
feeds an incremental UTF-8 decoder shared by DICP and Bidi_Control observers.
The independent normalization oracle passes 9 focused tests and exposes
400,680 official transformations, 1,121,256 assigned-scalar identity cases,
and 3,258,656 unassigned-scalar identity cases. No production normalizer or
normalization finding exists.

## Design

Retain exact input bytes fallibly inside the existing authoritative read loop
only while UTF-8 can still be valid. Continue hashing, counting, decoding, and
existing observations through EOF. A buffering failure is remembered while
the read continues and ultimately fails the command. Malformed or incomplete
UTF-8 discards all normalization-prefix state and produces only the frozen
invalid evidence.

For valid UTF-8, validate the retained bytes as one complete `str` without
unsafe code. Run the pinned crate's whole-sequence NFC and NFKC iterators. Do
not use Quick_Check as the decision, StreamSafe, or complete normalized
`String` values. Independent iterator passes compute the exact normalized
UTF-8 digest, checked byte/scalar counts, sequence identity, and bounded first
scalar divergence. The dependency may retain an arbitrarily long canonical
segment for pathological non-starter runs; scrub adds no unjustified fixed
window.

Keep the two findings independent and rely on existing canonical report sorting
for mechanism and evidence ordering. NFD and NFKD are exercised only by the
exhaustive conformance test.

## Acceptance criteria

- Exact pinned dependency identity, licenses, transitive tree, MSRV, release
  state, APIs, Unicode data version, allocation behavior, and upstream test
  generation are audited and recorded.
- NFC/NFKC statuses and evidence exactly match the frozen contract for valid
  identity, valid difference, and invalid UTF-8.
- `first_difference` covers all frozen offset, end, expansion, contraction,
  reordering, eight-scalar, and truncation cases.
- A scrub-specific corpus covers every named Milestone 4B-2 failure mode,
  arbitrary reader partitioning, both distinct real 65,536-byte cases, late
  malformed input, determinism, and input preservation.
- Production passes all 4,780,592 independent normative comparisons, and a
  test pins `unicode_normalization::UNICODE_VERSION == (17, 0, 0)`.
- Compiled CLI tests freeze mechanism/evidence order, raw JSON order, human
  output, complete artifact identity, malformed behavior, and repeated bytes.
- Lightweight large/pathological input measurements find no unintended
  superlinear scrub-owned work or duplicate full normalized strings.
- Public conformance and source-ledger documentation matches tested reality.

## Implementation steps

1. Audit and pin the production dependency.
2. Add whole-sequence normalization analysis to the existing read path.
3. Add direct semantics, first-difference, partition, boundary, invalid-input,
   compiled CLI, determinism, and non-modification tests.
4. Execute the independent official, assigned, and unassigned conformance
   domains for all four normalization forms.
5. Measure large and pathological inputs, then update documentation and the
   source ledger only after production tests pass.
6. Run all repository quality, YAML, diff, authored-file, and Git-index gates.

## Validation

- Focused oracle, production, corpus, conformance, and compiled CLI tests.
- Exact 400,680 + 1,121,256 + 3,258,656 comparison counts and runtime.
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- YAML parse and source-ID uniqueness.
- `git diff --check`, complete authored-file reads, status, and empty-index
  checks.

## Risks / open questions

- The dependency's normalizers use growable `TinyVec` canonical-segment
  buffers. Arbitrarily long non-starter input therefore has input-dependent
  memory and sorting cost, as UAX #15 permits. Measurements must characterize
  this honestly; scrub must not insert CGJ or claim constant memory.
- Rust's allocator may abort the process on an allocation failure inside a
  dependency. Such an abort is a command failure and emits no finding; scrub's
  own whole-artifact buffer uses fallible reserve so its failure is typed.
- Full conformance remains in ordinary testing only if measured runtime is
  compatible with normal development iteration; otherwise it becomes an
  explicit mandatory `just conformance` gate.

## Outcome

Production now emits the two frozen Unicode 17.0.0 NFC- and NFKC-difference
findings. The existing 65,536-byte read loop remains authoritative for complete
artifact SHA-256, length, UTF-8 validation, DICP, and Bidi_Control. While UTF-8
remains potentially valid, that loop retains the exact bytes with fallible,
amortized `Vec` growth. Definite invalidity discards that buffer; EOF-incomplete
UTF-8 is also invalid. A scrub-owned buffer allocation failure, arithmetic
failure, or internal UTF-8 inconsistency fails the command and cannot become a
finding.

For valid input, safe `from_utf8` creates a borrowed whole-sequence view.
`unicode-normalization` iterators produce NFC and NFKC independently. Separate
iterator passes compare scalar identity, hash exact normalized UTF-8 bytes,
count checked UTF-8 bytes and scalars, and construct the bounded first scalar
divergence. No complete normalized `String`, Quick_Check decision, StreamSafe
adapter, or CGJ is used. The original artifact is never opened for writing.

The selected exact dependency is `unicode-normalization` 0.1.25, crates.io
checksum `5fd4f6878c9cb28d874b009da9e8d183b5abc80117c40bbd187a1fde336be6e8`,
from the crates.io registry and upstream release commit
`5a69b3bafb625caccdec7871a42bed0d9a6604d1`.
It is unyanked, was published 2025-10-30, declares Rust 1.36, and is MIT OR
Apache-2.0. Its generator and generated public constant both pin Unicode
17.0.0; the generator loads official `NormalizationTest.txt`. Its production
tree adds only `tinyvec` 1.10.0 and `tinyvec_macros` 0.1.1, both under
permissive MIT/Apache-2.0/Zlib choices. The dependency's decomposition and
recomposition iterators retain growable canonical segments, which is required
for arbitrary non-starter runs and prevents a constant-memory claim.

The public `UnicodeNormalization` trait exposes lazy `nfc`, `nfd`, `nfkc`, and
`nfkd` `char` iterators used by the production and conformance paths. The
separate Quick_Check API exposes an indeterminate `Maybe` result and its
`is_nfc`/related helpers resolve that result by full normalization; production
does not use Quick_Check as its decision. The `stream_safe` adapter can insert
U+034F CGJ and is not used. Internally, decomposition and recomposition use
inline-then-growable `TinyVec` buffers for canonical segments; scrub adds one
fallibly grown exact-input buffer and bounded evidence, but cannot make the
dependency's own allocator failures recoverable.

The generator has explicit code to turn the official normalization file into a
generated test table, but neither that generated table nor the official text
file is present in the published crate archive or exact release commit tree;
scrub therefore relies on its own committed complete official oracle rather
than treating upstream packaged tests as conformance evidence. The release is
the current unyanked crates.io version and its source contains active
stable/beta/nightly plus Windows/Linux CI; this is adequate maintenance evidence
for one small pinned dependency, not a promise about future maintenance.

The untouched independent oracle graded production across all 400,680 official
transformations, 1,121,256 assigned-scalar identity cases, and 3,258,656
unassigned-scalar identity cases: all 4,780,592 comparisons passed. Individual
final focused debug-test timings were 1.636 s, 3.474 s, and 8.531 s; the whole
command took 10.816 s including compilation. The complete conformance target
finished in 8.41 s during the final workspace run because tests execute
concurrently. This cost is retained in ordinary `cargo test`; no separate or
ignored gate was needed. The public dependency version guard asserts
`unicode_normalization::UNICODE_VERSION == (17, 0, 0)`.

A 22-case compiled-CLI corpus freezes independent literal expected sequences,
statuses, digests/counts, first-difference JSON, real reader boundaries, long
non-starter behavior, and neutral output. Additional tests cover all established
malformed UTF-8 variants, a sensitive valid prefix followed by malformed data,
malformed data crossing the real boundary, late invalidity after three reads,
arbitrary trailing bytes in complete artifact identity, arbitrary one-byte and
irregular `Read` partitions, byte-identical repeated JSON/human output, and
input preservation.

Local release-build sanity measurements used one process per generated input
and polled Windows `PeakWorkingSet64`: 8,388,608 ASCII bytes took about 1,534 ms
and 12.8 MiB; 9,000,000 already-normalized multilingual bytes took 1,365 ms and
20.8 MiB; 3,000,000 normalization-sensitive bytes took 261 ms and 8.8 MiB; a
400,003-byte sequence containing 200,000 non-starters took 170 ms and 7.6 MiB;
and 900,000 compatibility-expansion-heavy bytes took 327 ms and 5.7 MiB. These
are authoring-time observations on one Windows machine, not benchmark claims.
The implementation holds one complete input buffer plus normalizer segment
state and bounded report evidence; it does not build duplicate normalized
strings.
