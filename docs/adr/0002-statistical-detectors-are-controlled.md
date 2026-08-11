# ADR 0002: Statistical watermark detectors are controlled mechanisms

**Status:** accepted

## Context

Many generative text watermarks require scheme-specific information such as a
reference implementation, tokenizer/model revision, key/configuration, calibrated
threshold, text length, or other prerequisites. Applying unrelated detectors to
arbitrary pasted text and reporting a generic probability is not sound evidence.

Anthropic has announced model-level Claude text watermarking for supported models,
but scrub.ts does not currently possess a supported third-party detector with the
required public prerequisites.

## Decision

1. `scrub inspect` does not attempt universal statistical watermark inference.
2. Statistical detectors may run only when their required prerequisites are known
   and represented in the analysis context.
3. Detector output must retain calibration/threshold assumptions needed for
   interpretation.
4. Controlled generation, attack, calibration, and comparison experiments live
   in WaterLARP.
5. Until a supported Claude detector exists, Claude statistical text-watermark
   status is `UNKNOWN` when the question is requested/relevant.

## Consequences

- A negative KGW, SynthID, Unigram, or other detector does not imply a negative
  Claude watermark result.
- Absence of hidden Unicode does not imply absence of a model-level watermark.
- Future Claude support requires a new/updated spec and source ledger entry before
  implementation.
