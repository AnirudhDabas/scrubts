# Milestone 5: first production C2PA provenance inspection

## Goal

Ship layered, inspection-only C2PA 2.4 evidence for Appendix A.8 unstructured
text carriers and released-SDK PNG, JPEG, and SVG validation without collapsing
carrier presence, manifest integrity, hard binding, or credential trust.

## Non-goals

Claude embedded-text-watermark detection, authorship detection, mutation or
stripping, C2PA A.7/A.9, PDF, office formats, remote manifests, sidecars,
network, live trust lists, soft binding, CAWG identity UX, signing, generation,
shell tools, plugins, GUI/web, and Git history operations are excluded.

## Sources / authority

- C2PA Technical Specification 2.4, exact ignored archive identity recorded in
  `docs/specs/c2pa-provenance.md` and `research/sources.yaml`.
- C2PA validation status-code definitions in that specification.
- released `c2pa` 0.90.12, crates.io checksum
  `0bcd2a168e8ce506789d4e5a66c286e5aa4944bc2181d75360b3ddf723ac4264`,
  tag `c2pa-v0.90.12`, commit
  `ae0c3fde8ea399bf7f12379bb44e38b2738b8369`, published 2026-08-12.
- official fixtures from that exact c2pa-rs revision under MIT OR Apache-2.0.
- Anthropic's official marking documentation, rechecked 2026-08-12.
- `c2pa-text` 3.0.0 (MIT) only as an authoring-time mapping/byte-order
  cross-check.

## Current state

HEAD `bfdc53b3c520285cae9b839cdf51a7ab6d51560a` has four production Unicode
findings and one authoritative 65,536-byte identity/decode loop. Pre-flight
`just check` passed offline with 85 workspace tests and all 4,780,592
normalization comparisons. The index was empty. Historical untracked
seed/release/research files were present and are unrelated.

An independent review subsequently returned **FAIL** with exactly four required
fixes: enforce the Appendix A.8 zero-or-one wrapper quantity; prevent
untimestamped embedded/stapled OCSP and CertificateStatus processing from
making public bytes depend on wall clock; recognize valid SVG prologs with
arbitrary processing instructions; and make fixture metadata agree with its
claim that Git blob OIDs are recorded. This blocker-fix pass is limited to
those findings and semantics directly changed by them.

The targeted independent re-review of that four-blocker pass returned
**FAIL** with three remaining blockers: Phase 2 authorization used an
incomplete public Manifest map; the clock regression did not inject controlled
instants into c2pa 0.90.12 itself; and the SVG scanner accepted invalid or
reserved XML PITargets. This final bounded pass addresses only those three
findings and their direct regressions. Final targeted re-review remains
outstanding.

## Dependency decision

Use exact `c2pa = "=0.90.12"`, `default-features = false`, features
`["rust_native_crypto"]`. The crate is pre-1.0 beta software, declares Rust
1.88, and is MIT OR Apache-2.0. Default features are rejected because they add
OpenSSL and four HTTP integrations. `rust_native_crypto` provides the required
released verification path without accidentally selecting vendored/system
OpenSSL. The lockfile, local `.crate` SHA-256, complete normal dependency tree,
and transitive licenses will be recorded in Outcome after resolution.

Release 0.90.12 contains the security-relevant fix “Validate inputTo
ingredients against manifest tampering” from PR #2476, plus a SHA-1 dependency
update and identity split-signing preservation. Adjacent 0.90.11 prevents a
panic on out-of-range GeneralizedTime conversion. Tests should preserve the
invariant that an outer parse cannot turn a tampered relationship into valid.

The released feature list has no `unstable_plain_text` or
`unstable_structured_text`. Upstream A.8 issue #2424, PR #2117, and A.9 issue
#2377 remain open/experimental. No open PR, `main`, or `c2pa-text` production
dependency is allowed.

## Design

### Same-byte path

Extend the existing read loop with an eight-byte fixed prefix sniffer. Upon an
exact PNG or JPEG signature, fallibly retain the prefix and every subsequent
read slice. For complete valid UTF-8, reuse the existing retained bytes for SVG
or A.8. Hashing, byte count, Unicode observers, and retention consume the same
slices once. Validation borrows the final retained bytes through `Cursor`; no
path reopen or second artifact read occurs.

### A.8 boundary

Implement one scrub-owned scalar scanner limited to U+FEFF, the two variation
selector ranges, C2PATXT magic, version, big-endian u32 length, exact physical
payload bounds, and bounded evidence hashing. Use checked arithmetic and no
attacker-sized allocation. Preserve DICP observations independently. Enforce
the normative zero-or-one quantity and emit
`manifest.text.multipleWrappers` for the second valid wrapper.

Released 0.90.12 cannot parse/validate an extracted A.8 Manifest Store with
text semantics. Its `.c2pa` handler describes an external manifest-store asset
and would apply no A.8 content binding. Therefore A.8 `manifest_store`,
`manifest_validation`, and `hard_binding` are `unsupported`; trust follows the
frozen contract. Full A.8 cryptographic tests are deliberately not fabricated.

### Binary SDK path

Use SDK content detection for PNG/JPEG and scrub's bounded XML-root recognition
for SVG, including processing instructions with XML 1.0 Fifth Edition Name
targets before the root, then call only in-memory SDK APIs with an explicit
Context. Extract the
embedded JUMBF bytes through the structured SDK API for presence/digest. A
verification-disabled Reader establishes structural store presence and stable
timestamp/CertificateStatus metadata. A separate validation Reader consumes a
second Cursor over the same retained bytes only when a recursive walk from the
public active label proves every public ingredient manifest reference resolves,
every reached Manifest has a validated timestamp, and none has
CertificateStatus. Match typed SDK errors and structured validation/status
APIs; never inspect human-readable error text or SDK JSON.

### Isolation

Build without file or HTTP features. Replace all context settings explicitly,
block all allowed network hosts, disable remote fetching and OCSP, disable
trust and timestamp-trust decisions, clear trust material, and disable CAWG
identity decoding. Tests run beside a same-basename `.c2pa` file under hostile
environment values and prove identical output.

### Finding contract

The five IDs, status precedence, evidence keys/order, 256-code bound,
normalized offset semantics, trust boundary, and exact nine-finding order are
frozen in `docs/specs/c2pa-provenance.md` before production implementation.

## Fixtures and test matrix

Freeze at least one official known-good asset for each binary format from the
exact c2pa-rs release when its license and validation characteristics permit.
Record upstream path, Git blob, byte length, SHA-256, and expected structured
statuses. Supplement with unsigned minimal files and independently specified
byte mutations. Fixture helpers must not generate their own expected values.

Required focused coverage includes:

- the complete requested A.8 absence, valid-shape, corruption, multiple,
  reader-boundary, malformed UTF-8, and Unicode-coexistence corpus;
- decomposed, BMP, and supplementary prefixes proving NFC UTF-8 coordinates;
- PNG/JPEG/SVG absent, valid, store/signature/binding tamper, extension mismatch,
  identity, UTF-8 independence, 65,536 boundary, repeatability, and preservation;
- raw JSON and human output order, empty success stderr, and prohibited claims;
- ambient settings, sidecar, offline build/run, no textual-message false
  positive, bounded status extraction, and recent inputTo regression where a
  practical official fixture/API permits it.

## Acceptance criteria

- All five layers report only their documented question and status.
- Same bytes determine top-level identity and C2PA validation.
- No network, sidecar, neighbor, ambient config, OS trust, or path reopen can
  affect output.
- Binary C2PA remains independent of malformed UTF-8.
- A.8 carrier parsing is bounded and full downstream validation remains honest.
- Official fixtures and derivatives have independent provenance/identity.
- Focused tests, compiled CLI tests, offline workspace tests, formatting,
  Clippy, source-ledger checks, `git diff --check`, and empty-index checks pass.

## Implementation steps

1. Freeze source identities, dependency choice, public contract, and this plan.
2. Resolve/audit the exact minimal dependency and record its actual tree.
3. Implement classification, same-byte retention, A.8 carrier inspection, and
   explicit finding order.
4. Integrate binary SDK extraction/validation and deterministic status mapping.
5. Add official fixtures, hostile derivatives, focused and real-CLI tests.
6. Measure release-mode sanity cases, update README/conformance/source ledger,
   and complete all offline quality and Git-state gates.

## Risks / stop conditions

Stop rather than weaken semantics if explicit Context replacement cannot isolate
ambient/network behavior; if same-byte validation cannot be maintained; if a
license conflicts; if a normative A.8 detail remains unresolved; if full A.8
requires JUMBF/CBOR/COSE reimplementation; or if the report cannot preserve the
layer distinctions. Untimestamped SDK validity is reported `unknown` rather
than allowing wall-clock-dependent canonical output.

## Outcome

Implementation is complete in the unstaged worktree. Independent review remains
outstanding; this outcome does not claim that review passed.

The first independent review's four required fixes were applied in the unstaged
worktree: A.8 now rejects multiple valid wrappers; binary store parsing is
separated from validation and non-reproducible assurance paths are `UNKNOWN`;
the SVG prolog scanner accepts arbitrary terminated processing instructions;
and every imported C2PA fixture now records a verified upstream Git blob OID,
byte length, SHA-256, path, revision, and license. The targeted re-review then
failed on the three bounded blockers recorded in Current state.

Those three fixes are now applied in the unstaged worktree. Phase 2 starts at
the stable public active label and recursively follows public ingredient active
manifest labels; every reached label must materialize through
`Reader::get_manifest` before timestamp and CertificateStatus checks authorize
validation. A frozen test-only derivative proves an untimestamped referenced
Store claim omitted after `Manifest::from_store` failure cannot be hidden by a
timestamped active Manifest. A disposable script verifies the cached c2pa
0.90.12 archive identity, patches only a temporary extracted copy with a
controlled test clock, executes the actual certificate-profile and OCSP
decision paths at before/after instants, and deletes that source copy. The SVG
scanner now implements XML 1.0 Fifth Edition `Name` for PITarget and rejects
case-insensitive `xml` outside the XML declaration path. Final targeted
re-review remains outstanding. This is not an independent PASS claim.

The selected crate and local downloaded `.crate` both have SHA-256
`0bcd2a168e8ce506789d4e5a66c286e5aa4944bc2181d75360b3ddf723ac4264`.
The resolved Windows normal tree contains 244 unique packages (299 when all
target-conditioned normal packages are included). No `reqwest`, `ureq`,
`hyper`, `curl`, `isahc`, or `openssl` package is selected. Every package in
the resolved normal tree declares a license; the expressions resolve through
MIT, Apache-2.0, BSD variants, Unicode-3.0, Zlib, BSL-1.0, 0BSD, or Unlicense.
The direct C2PA verification graph includes native Rust P-256/P-384/P-521,
ECDSA, Ed25519, and RSA implementations. The graph is materially large, but it
is bounded to the released SDK path and avoids HTTP and signing-generation
features.

Production now emits the five frozen C2PA findings after the four unchanged
Unicode findings. PNG/JPEG bytes are retained fallibly from the authoritative
65,536-byte identity loop; SVG and A.8 reuse its valid-UTF-8 retention. The SDK
receives only borrowed cursors over those bytes and an explicitly constructed
Context. The clean-environment integration test places a same-basename
`.c2pa` sidecar beside an unsigned PNG, supplies hostile C2PA environment values
and failing HTTP proxies, and obtains byte-identical output to the baseline.

The independent known-good fixture is the C2PA public-testfiles
`adobe-20220124-CA.jpg` at commit
`22beccc075707475b038d8789d0136c009e43143`, CC-BY-SA-4.0: 178,709 bytes,
SHA-256 `cafc48c53e651f7ba4622d1f72783827074211e42b9634cc863ec3be3c7651b3`.
Three official tamper vectors cover data, signature, and claim/ingredient
relationship changes. Generated PNG/SVG fixtures use the selected SDK and its
upstream non-production test credential only to supplement—not replace—the
independent JPEG authority. Exact fixture paths, identities, licenses, and
generation boundary are frozen in `crates/scrub/tests/fixtures/c2pa/README.md`.
Tests additionally make independently specified PNG IDAT mutations with a
recomputed CRC, mutate embedded PNG/SVG manifest bytes, and truncate stores.

The A.8 parser is scrub-owned and limited to the normative carrier. Carrier
tests cover absence, wrong and truncated magic, version and big-endian length,
supplementary selectors, interruption, mutation, multiple candidates, a
declared `u32::MAX` length, 200,000 non-candidate U+FEFF values, a 300,000
selector non-magic run, a one-MiB payload, arbitrary Reader partitions, and the
actual read boundary. The decomposed-prefix test freezes original UTF-8 wrapper
offset 8 and NFC UTF-8 offset 7 after `Ae\u{301}😀`; ASCII, BMP, and
supplementary prefixes are also covered. Released 0.90.12 cannot validate the
extracted A.8 store with text semantics, so all downstream A.8 cryptographic
layers remain explicitly `unsupported` and no hard-binding success is claimed.

Warm release-mode process observations on this Windows host (three runs after
one warm-up) were approximately: 10 MiB no-FEFF text 1.93–2.13 s; 200,000
FEFF non-candidates in 0.80 MiB 137–154 ms; a 300,000-selector non-magic run in
0.90 MiB 125–162 ms; a one-MiB A.8 payload encoded as 3.15 MiB UTF-8 328–332
ms; an unsigned 5.54 MiB PNG 54–61 ms; the 178,709-byte signed JPEG 53–54 ms;
and a 5.42 MiB data-tampered JPEG derivative 81–87 ms. A polling observation
for the longer plain-text run saw about 15 MiB peak working set; the host API
returned no reliable post-exit peak for the shorter runs. These are authoring
sanity observations, not published benchmarks or performance claims.

The final offline workspace run executes 116 tests: 34 scrub unit tests, 15
C2PA compiled-CLI tests, 58 existing Unicode/CLI integration tests, and 9
scrub-report tests. The 14 focused carrier tests and all 4,780,592 Unicode
normalization oracle comparisons pass. Formatting, Clippy, final `just check`,
source-ledger parsing/uniqueness, and Git whitespace/index checks are recorded
in the review handoff after their final rerun.

Unsupported scope remains A.8 cryptographic/hard-binding validation, A.7/A.9,
all formats beyond PNG/JPEG/SVG/text, remote/sidecar manifests, network/OCSP,
trust-list decisions, soft bindings, mutation, and vendor watermark detection.
