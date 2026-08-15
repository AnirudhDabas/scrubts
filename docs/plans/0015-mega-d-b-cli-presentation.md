# Mega D-B: launch-quality human CLI presentation

## Goal

Make the existing human `scrub inspect` projections immediately distinguish
observations, statuses, evidence, verifier authority, supported inferences, and
unsupported inferences without changing the typed `Report` or inspection
behavior.

## Non-goals

- New detectors, evidence, report fields, ontology, provider registry, research
  claims, C2PA behavior, WaterLARP work, fuzzing, benchmarks, or dependencies.
- JSON/schema, mechanism, status, authority, inference, claim-ledger, proof, or
  production scanner changes.
- Color, terminal-capability detection, decorative banners, or demo data.

## Sources / authority

`AGENTS.md`, `docs/SOURCE_AUTHORITY.md`, ADR 0001,
`docs/specs/report-schema.md`, `docs/specs/product-proof.md`, Mega A's typed
trace contract, and Mega D-A's controlled U+200B artifact govern this work. The
existing `Report` remains the only semantic source for every projection.

## Current state

At `f49ba96401d2572e315e1ddd26cf8dad6a52f78e` on `main`, the tracked
worktree and index are clean. Nine unrelated untracked human-owned files are
outside this milestone. Default output preserves the Unicode PRESENT and Claude
UNKNOWN distinction but presents evidence as compact JSON. Explain output
concatenates `does not support` with its first value because the shared
13-character label column is narrower than that label. Long statuses are also
not guaranteed an explicit separator by the default summary format.

## Design

1. Keep one deterministic, plain-text renderer with no environment-dependent
   styling. Present Artifact, Observations, and Interpretation as the default
   hierarchy.
2. Group Unicode, C2PA, and Claude observations. Retain report status names and
   humanize only the renderer's deterministic Unicode location projection;
   `--explain` retains the raw evidence values.
3. Show the unavailable Anthropic verifier and public-SynthID related-reference
   boundary directly under Claude UNKNOWN. State concisely that Unicode
   evidence does not establish an Anthropic watermark, Claude involvement, or
   authorship, and that UNKNOWN is not ABSENT/CLEAN.
4. Route explain fields and inference lists through one explicit 18-character
   label column. Render status names in uppercase and split authority classes
   and source IDs onto aligned continuation rows for scanability.
5. Continue applying the existing terminal-safe escaping projection to every
   report-originated string rendered by the changed paths.

## Acceptance criteria

- The controlled fixture visibly reports U+200B at byte and scalar offset 4 as
  Unicode PRESENT, separately from Claude embedded-text-watermark UNKNOWN.
- Default output exposes unavailable Anthropic verifier authority, the public
  SynthID non-parity boundary, and the key inference boundary without prose
  walls.
- Explain output contains `does not support` and `artifact_clean` as separated,
  aligned tokens and retains the complete evidence/authority/inference trace.
- Human and explain output are deterministic and terminal-safe.
- `--json` and `--json --explain` remain byte-identical; the controlled
  fixture's JSON remains byte-identical to the pre-change capture.

## Implementation steps

1. Adjust only the CLI renderer and its small formatting helpers.
2. Update focused CLI and complete-output regressions for the new projection.
3. Capture the controlled fixture's default, explain, and JSON outputs; compare
   JSON bytes against the pre-change baseline.
4. Run focused tests, repository gates, proof, and the final Git/scope audit.

## Validation

Run focused `scrub` CLI and normalization tests first, then:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
just check
just prove
git diff --check
```

Inspect the exact controlled-fixture default and explain commands requested by
the milestone, compare JSON bytes and parsed semantics, and audit Git state
without staging or history operations.

## Risks / open questions

- Humanizing location evidence must fail closed to the escaped raw value if it
  does not exactly match scrub's deterministic location encoding.
- The renderer has no terminal-width negotiation. This milestone uses short
  default lines and aligned explain continuations without adding
  environment-dependent behavior.

## Outcome

Implemented the bounded plain-text presentation in the existing CLI renderer.
Default output now separates Artifact, grouped Observations, and Interpretation;
the controlled fixture renders `U+200B at byte offset 4, scalar offset 4` under
Unicode `PRESENT`, while Claude remains a separate `UNKNOWN` observation with
the unavailable Anthropic verifier and public-SynthID non-parity boundary.
Explain output retains all raw evidence and trace values, uses uppercase status
names, splits authority/source lists into aligned continuation rows, wraps
limitation prose deterministically, and gives `does not support` an explicit
18-character field so it cannot collide with `artifact_clean`.

Focused renderer, CLI, terminal-safety, C2PA, normalization snapshot, and
determinism tests passed. The final requested gates passed: `cargo fmt --check`,
warning-denied workspace Clippy, `cargo test --workspace`, `just check`, and
`just prove`. Proof completed with all existing 16 claims passing and
`PROOF_COMPLETE`. The controlled fixture's `--json` and `--json --explain`
outputs were byte-identical to each other and to the pre-change capture:
10,452 bytes, SHA-256
`2c88c719b32985af3f1ab2fc01350d5aacc33bf22e87c4f53889cd91c2c3cf07`.

No report type/value, JSON/schema, mechanism/status, authority/inference,
scanner, C2PA, Anthropic, SynthID, WaterLARP, claim-ledger, proof-claim, source,
dependency, staging, or Git-history behavior changed. The renderer remains
deterministic and terminal-safe; no color or terminal-capability dependency was
introduced.
