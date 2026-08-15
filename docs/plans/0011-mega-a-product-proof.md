# Mega A: product and proof architecture

## Goal

Make `scrub inspect`, `scrub inspect --explain`, `scrub inspect --json`, and
`just prove` deterministic projections of auditable typed evidence. Preserve the
existing Unicode, C2PA, and WaterLARP scientific boundaries while adding a
versioned machine contract, provider-authority UNKNOWN handling, and an offline
claim proof artifact.

## Non-goals

- New detectors, watermark schemes, C2PA behavior, WaterLARP experiments, GPU
  work, networking, releases, fuzzing, website work, or broad polishing.
- RFC 8785/JCS compliance, cross-platform digest claims, or reliance on ignored
  local pilot outputs.
- Git staging or history operations.

## Sources / authority

`AGENTS.md`, `docs/SOURCE_AUTHORITY.md`, ADRs 0001--0003, the existing report,
Unicode, C2PA, and WaterLARP specifications, `research/sources.yaml`, and the
frozen authority records in `waterlarp/src/waterlarp/authority.py` govern this
work. Mega A does not reinterpret the closed WaterLARP milestone.

## Current state

HEAD `80e31532179be8658d3f0a4c33c99b8bad885121` is the reviewed WaterLARP
milestone on `main`; the index is empty. Nine pre-existing untracked files are
outside this milestone. `scrub-report` schema 0.1 already has a closed status
enum, stable ordered structs, artifact SHA-256 identity, and nine Unicode/C2PA
findings. `scrub` builds one report and renders either verbose human text or
single-line JSON. There is no report JSON Schema, typed inference/authority
trace, Claude UNKNOWN finding, claim ledger, or proof command.

## Design

1. Extend the existing `Report` to schema 0.2. Each `Finding` carries one typed
   `ProofTrace`: verifier identity/availability, authority classes and source
   IDs, optional configuration identity and related-reference boundary, typed
   supported and forbidden inference IDs, and a reproducible command template.
   Scanner/provider construction supplies this trace once; renderers never
   infer or upgrade it.
2. Keep display artifact context in the report but expose
   `canonical_report_bytes` as a path-free, sorted semantic projection. This is
   scrub-specific deterministic JSON, not RFC 8785. Full JSON remains one
   versioned report with structured traces; `--json --explain` uses that same
   document.
3. Render a concise grouped default view and a deterministic detailed explain
   view. Add a non-runnable Anthropic finding for textual artifacts whose status
   is always UNKNOWN under the frozen authority record. Public SynthID remains
   only a related reference and cannot satisfy provider authority.
4. Add Draft 2020-12 report, claim-ledger, and proof schemas. The canonical
   `evidence/claims.json` records only real repository evidence and executable
   offline reproduction commands.
5. Add one small standard-library Python orchestrator behind `just prove`. It
   executes the ledger's required commands without a shell, fails closed, emits
   a human summary, and writes `target/mega-a/proof.json`. It may use the local
   WaterLARP virtual environment when present but performs no network access.

## Acceptance criteria

- Human, JSON, explain, and semantic bytes derive from one report; UNKNOWN
  cannot render as clean/human and public-reference evidence cannot become
  provider evidence.
- JSON is schema 0.2, parseable, ANSI-free, deterministically ordered, free of
  absolute input paths and incidental host/time state, and locally repeatable.
- The claim ledger has unique real claims, valid evidence/source references,
  and executable reproduction commands.
- Every required proof PASS comes from a successful oracle; any failure yields
  non-zero and an honest schema-valid `proof.json`.
- Existing scientific gates remain unchanged and pass.

## Implementation steps

1. Add proof-carrying report types, semantic bytes, provider finding, renderers,
   CLI flags, schemas, and focused tests.
2. Add the claim ledger, proof orchestrator/tests, and `just prove`.
3. Update only report/proof contract documentation.
4. Run narrow tests, full gates, proof/schema/repeatability checks, and one
   focused Mega A semantic review; record the measured outcome here.

## Validation

Run focused `scrub-report`, CLI, schema, proof, authority, Unicode, and C2PA
tests first; then `cargo fmt --check`, warning-denied Clippy, `just check`,
`just prove`, independent proof-schema validation, deterministic repeat checks,
and `git diff --check`.

## Risks / open questions

- Full report JSON contains a display name; only the explicit semantic
  projection excludes display context. Mega B must test the digest boundary on
  Windows, Linux, and macOS before any cross-platform claim.
- The KGW committed fixture can be identity-checked offline, but independent
  upstream parity still needs the pinned checkout and is not a default proof
  claim.
- Ignored WaterLARP pilot results are not a fresh-clone proof dependency and no
  canonical-pilot PASS row will be invented.

## Outcome

Implemented schema 0.2 as one proof-carrying `Report`. Existing scanner calls
still construct the finding once; `scrub-report` attaches the frozen verifier,
authority/source, configuration, supported inference, forbidden inference, and
reproduction trace. Report decoding rejects a trace that does not match the
mechanism/status ontology. Default human, explain, JSON, and path-free semantic
bytes are projections of that report. Textual artifacts now carry the frozen
Anthropic `UNKNOWN` provider slot; public-reference SynthID is represented as a
distinct authority and cannot satisfy it.

The Draft 2020-12 report, claim, and proof schemas live in `schemas/`.
`evidence/claims.json` contains 12 real claims/gates. `just prove` executes their
offline oracles without a shell and writes schema-valid
`target/mega-a/proof.json`. The final warm run passed all 12 gates in 44.8
seconds; the first cold run took 89.1 seconds. The artifact contains one local
report digest scoped explicitly to local repeatability. No canonical proof
digest is emitted. Its observed final file SHA-256 was
`55a1e997131b87c3ae5abfce23028760140101b882475e179f29bb70ead02c6f`,
which is file integrity information rather than a semantic proof identity.

Focused tests passed for report semantics, provider/reference separation,
C2PA's presence/validity/binding/trust ladder, imported trace-upgrade rejection,
human/explain/JSON projections, display-path/control exclusion, semantic-byte
repeatability, schema validation, claim resolution, and proof fail-closed
behavior. `just check` passed formatting, warning-denied Clippy, and 125 Rust
tests, including all 4,780,592 normalization oracle comparisons. The three
proof-orchestrator unit tests passed. Independent Draft 2020-12 validation of
the generated proof reported `PROOF_COMPLETE`, 12 gates, and one locally scoped
report digest. `git diff --check` passed with only Git's existing LF/CRLF
working-copy notices.

The focused self-review found and fixed two blockers: imported JSON could have
carried a schema-valid but ontology-inconsistent authority trace, and the first
concise renderer collapsed an applicable absent C2PA text-wrapper result into
an aggregate not-applicable line. It also corrected the Unicode property source
mapping and added an explicit public-SynthID/provider separation regression.

No dependency, scanner algorithm, C2PA behavior, WaterLARP scientific semantic,
source pin, ignored pilot artifact, staging area, or Git history changed.
Cross-platform digest comparison, terminal-injection attack hardening beyond
the new renderer's control escaping, and live pinned KGW parity remain deferred
to their stated milestones.
