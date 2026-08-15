# WaterLARP post-review blocker fixes

## Goal

Repair the nine launch-blocking scientific and reproducibility failures found by
the 2026-08-14 independent methodology review, then produce a fresh CPU
integration pilot whose canonical records are independently auditable without
the generation checkpoint.

## Non-goals

- New watermark families, provider attribution, or Claude detection.
- Bayesian SynthID, attack implementations, semantic operation models, or
  paper/launch-scale execution.
- Generated MBPP execution without a hostile-code sandbox.
- Website, release, chart, or marketing work.
- Git staging or history operations.

## Sources / authority

`docs/SOURCE_AUTHORITY.md` governs. KGW behavior is pinned to
`jwkirchenbauer/lm-watermarking@82922516930c02f8aa322765defdb5863d07a00e`;
SynthID Text behavior is pinned to DeepMind
`addb4a158143c7c6851a1308f78b89fceed59683` and Transformers
`5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9`. The Anthropic catalyst was
rechecked against its official article on 2026-08-14 and remained inactive:
detector details are still forthcoming. The three corrected related-work
identities are frozen from ACL Anthology primary records and author-linked
GitHub repositories; they remain citation-only.

## Current state

HEAD is `088862e336fe0f0433a5460af71cbcc4d7f3386c`, the index is empty, and the
WaterLARP layer is unstaged. The existing gates pass (28 non-parity Python
tests, 30 with parity, 116 Rust tests, and 4,780,592 normalization cases), but
the implementation computes entropy from processor-mutated logits, represents
only one split member, pools calibration tasks, hardcodes calibration false
positives, omits shifted held-out negatives, leaks nominal length into
threshold lookup, trusts unbound checkpoints/caches, discards detector evidence,
and omits mixed-document held-out FPR/localization typing.

## Design

One content-addressed experiment model governs all fixes:

1. An experiment specification contains exact ordered split members and their
   cached-row hashes; dataset/model/tokenizer/prompt/source/lock identities;
   generation, detector/key, calibration, evidence-length, transform, and
   search configurations. Its canonical digest is `experiment_spec_id`.
2. A canonical JSON checkpoint embeds and verifies that specification identity,
   sample-set digest, model/tokenizer/generation identities, and a payload hash.
   The final `run_id` hashes the experiment specification ID together with the
   verified checkpoint payload digest, thereby binding generated token sequences.
3. Threshold records are task/model/tokenizer/key/config/evidence-policy and
   regime conditioned. Comparator semantics and actual calibration exceedances
   are serialized. Exact observable detector evidence length is the only length
   accepted by operational threshold lookup; unsupported lengths remain explicit.
4. Every example directly carries exact detector input token IDs, tokenizer and
   detector/key identities, raw detector evidence, scored-unit count, threshold
   provenance, comparator, and decision status. Window-search records additionally
   carry the full search specification and token-coordinate localization.
5. Operation-conditioned and mixed-document groups include independent transformed
   or searched test negatives. Aggregates derive held-out FP/N/FPR and exact 95%
   Clopper-Pearson intervals from those records.

## Acceptance criteria

- Pre-watermark entropy is captured from a non-aliasing base-logit snapshot in
  the actual autoregressive loop before any logits processor.
- Arbitrary-N exact split membership and cached-row identities round-trip,
  validate, and select experiment identity without leakage or duplicates.
- Calibration is task/config/key/evidence/regime specific, freezes comparator
  semantics, and records actual exceedances.
- Operation-conditioned and window-search results include independent held-out
  negative evidence and exact intervals.
- Threshold lookup uses only scored-unit count observed from detector input.
- Stale/tampered checkpoints, changed rows, and changed scientific identities are
  rejected or select a different content identity.
- Every pilot record can be rescored from canonical checksummed artifacts without
  reading the generation checkpoint.
- Localization uses explicit half-open `TOKEN` coordinates.
- Source ledger corrections are exact, tested, and citation-only.
- All requested Python, parity, schema, checksum, portability, Rust, Unicode, and
  diff gates pass; the paper plan remains unexecuted.

## Implementation steps

1. Replace generation observation and add in-place/KGW entropy regressions.
2. Implement sample-set/specification/checkpoint identities and portable lock.
3. Replace calibration and observable evidence-length contracts.
4. Promote canonical detector evidence and update result/manifest schemas.
5. Complete operation-conditioned and mixed-document held-out evaluation.
6. Correct source records and update scientific documentation.
7. Run narrow/adversarial tests, parity, a fresh CPU pilot, independent rescoring,
   schema/checksum/tamper/portability demonstrations, and complete repository gates.

## Validation

Use the commands required by the milestone request, with narrow pytest modules
first, then `just waterlarp-check`, full parity-enabled pytest, source verification,
a fresh pilot and aggregation, repository-root Draft 2020-12 validation, checksum
and canonical rescoring checks, checkpoint tamper/staleness tests, alternate-path
lock verification, `git diff --check`, and complete `just check`.

## Risks / open questions

- Tiny pilot cells cannot resolve 1% FPR; this must remain `UNRESOLVED` even when
  held-out false positives are zero.
- Exact evidence-length policy can yield `UNRESOLVED` for unseen post-edit scored
  lengths. This is intentional and must not be interpolated away.
- CPU generation time increases if the pilot uses two members per split. Reduce
  only pilot N if host limits require it; never weaken arbitrary-N contracts.

## Outcome

The nine-blocker architecture is implemented as one content-addressed contract.
The real CPU profile now requests two exact members per split, canonical
detector evidence is directly promoted, and the paper plan remains unexecuted.
Exact gate counts and the final fresh pilot identity are intentionally reported
in the maintainer handoff rather than copied into this plan, so this document
cannot become a stale duplicate of machine-readable results.
