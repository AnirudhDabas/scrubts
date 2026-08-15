# Mega D-A: Claude watermark claims audit

## Goal

Create one small, versioned, machine-readable audit that separates public
Claude-watermark claims from their observable signals, required authority, and
supported inference. Demonstrate the boundary with one project-created U+200B
fixture inspected by the existing production CLI.

## Non-goals

- Scanner, report-ontology, C2PA, Anthropic UNKNOWN, or public SynthID semantic
  changes.
- A provider detector, watermark remover, benchmark, WaterLARP experiment,
  public claim-ledger expansion, release change, visual, or launch copy.
- A census or prevalence estimate of public Claude-watermark pages.

## Sources / authority

- Anthropic, "How Claude's text watermark works," live representation captured
  2026-08-15. This is primary authority for Claude's stated mechanism family,
  hidden-character boundary, provider key, detector-API status, and inference
  limitations.
- Google AI for Developers, "SynthID: Tools for watermarking and detecting
  LLM-generated Text," live representation captured 2026-08-15.
- `google-deepmind/synthid-text` pinned at commit
  `addb4a158143c7c6851a1308f78b89fceed59683` as a public reference
  implementation.
- A six-source convenience sample recorded in the artifact's `sources.yaml`.
  Sample pages are authority for their own wording and implementation surfaces,
  not for Anthropic deployment behavior.
- Existing report semantics and source hierarchy in `docs/SOURCE_AUTHORITY.md`,
  `docs/specs/report-schema.md`, and schema 0.2.

## Current state

Baseline `24af24a72adc632852e5fd2114725b28bd3002f1` reports
`anthropic.embedded_text_watermark` as `UNKNOWN` for text because the checked
provider detector is unavailable. Unicode default-ignorable inspection is a
separate deterministic finding. No bounded Claude public-claims audit exists.

The 2026-08-15 Anthropic retrieval has a different HTML representation hash
from the prior source note, but the claims governing frozen provider semantics
remain substantively unchanged. This milestone records the new capture only in
its own source ledger and does not update frozen scanner or WaterLARP authority
records.

## Design

Add one strict JSON Schema and an artifact directory containing:

- seven claim records across six convenience-sample sources;
- a separate source ledger with short excerpts and exact live-capture or pinned
  repository identities;
- one deterministic U+200B controlled negative plus a separately stored visible
  rendering and construction metadata;
- raw production CLI JSON and explain output with a run manifest; and
- one offline verifier that validates schema and cross-file identities.

Classifications apply to individual quoted claims, not whole publishers or
tools. The ontology remains six classifications and five authority classes.

## Acceptance criteria

- Current Anthropic and primary SynthID sources are retrieved and encoded
  without remembered wording.
- The convenience sample contains 5-8 current sources with Unicode conflation,
  public mechanism demonstrations, and accurately limited counterexamples.
- Every claim and supporting source resolves and has a documented
  classification and authority boundary.
- The fixture contains exactly one injected U+200B at a documented position;
  its byte identity and visible companion match metadata.
- Stored CLI evidence is produced by the existing CLI and preserves its actual
  Unicode and Anthropic outcomes.
- `python tools/verify_claude_watermark_claims.py` deterministically validates
  the complete artifact and README table.
- No frozen scanner/report, WaterLARP, fuzz, determinism, release, or proof-claim
  semantics change.

## Implementation steps

1. Retrieve and compare current primary and convenience-sample sources.
2. Define the strict claim-audit schema and source/claim records.
3. Generate the controlled fixture and capture real CLI evidence.
4. Add cross-file verification and the concise research README.
5. Run focused verification, repository gates, and a claim-falsification pass.

## Validation

```text
python research/claude-watermark-claims/fixtures/generate_fixture.py
python tools/verify_claude_watermark_claims.py
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
just check
just prove
git diff --check
```

The production evidence commands are recorded exactly in the run manifest and
artifact README.

## Risks / open questions

- Live pages can change after capture; hashes identify retrieved
  representations but the pages themselves are not redistributed.
- Static page inspection may not reveal server-side detector behavior. Records
  state that limitation and avoid claims about unobserved services.
- A convenience sample cannot establish web-wide prevalence.
- Anthropic may later publish the announced API; this artifact is time-bounded
  and must not be treated as current without re-intake.
- The existing CLI's default-ignorable finding groups U+200B under the Unicode
  property rather than a vendor-specific label. That is semantically correct;
  any presentation follow-up belongs to Mega D-B.

## Outcome

Implemented the bounded artifact without modifying production scanner/report,
WaterLARP, fuzz, determinism, release, or existing proof-claim semantics. The
live Anthropic representation changed from the prior note's byte identity, but
the eight authority findings required here did not change the frozen provider
claim: the detector remains unavailable to scrub and the result remains
`UNKNOWN`.

The artifact contains seven claim records across six convenience-sample
sources, seven source records total, one 50-byte controlled U+200B fixture, raw
schema-0.2 JSON and explain output from the production CLI, and an offline
cross-file verifier. The observed controlled result is Unicode default
ignorable `PRESENT` and Anthropic embedded text watermark `UNKNOWN`.

Validation completed:

- `python tools/verify_claude_watermark_claims.py`: PASS;
- `cargo fmt --check`: PASS;
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS;
- `cargo test --workspace`: PASS;
- `just check`: PASS;
- `just prove`: `PROOF_COMPLETE`, 16/16 after the final verifier strengthening;
- `git diff --check`: PASS; an additional scan found no trailing whitespace in
  Mega D-A text artifacts.

One presentation issue is deferred: the existing explain renderer concatenates
`does not support` with its first inference. The stored JSON remains
unambiguous, and no semantic or renderer change is made in Mega D-A.
