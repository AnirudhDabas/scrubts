# scrub.ts

Evidence-first forensics for AI-generated text, code, and files.

`scrub` reports what it can verify, what it cannot, and why.

> **unknown ≠ clean**

## Status

Early research/engineering development. The initial scope is intentionally narrow:
standards-backed Unicode forensics, C2PA provenance inspection, and a separate
controlled research harness (**WaterLARP**) for public statistical watermark schemes.

Anthropic documents embedded, model-level text watermarking for supported
Claude-generated text.
Until scrub.ts has a supported detector with the required prerequisites, Claude
embedded text-watermark status is **UNKNOWN**, not absent.

## Principles

- Evidence before claims.
- Local and inspectable by default.
- Primary specifications and reference implementations over folklore.
- Reproducible experiments over screenshots.
- Calibration and sample-level results over context-free scores.
- Limitations documented beside capabilities.
- No universal AI-detection or watermark-removal claims.

## Current interface

```console
scrub inspect <path>
scrub inspect <path> --json
```

`scrub inspect` currently reports Unicode 17.0.0
`Default_Ignorable_Code_Point`, `Bidi_Control`, NFC-difference, and
NFKC-difference observations for one file. Normalization findings are neutral
comparisons against the untouched input; they do not rewrite text or establish
security risk, provenance, authorship, or watermark presence.

`scrub doctor` and broader mechanism families remain planned.

Watermark research lives separately in **WaterLARP** so experimental statistical
results cannot silently become production forensic claims.

Start with `docs/SOURCE_AUTHORITY.md`, `docs/specs/v0.1.md`, the ADRs under
`docs/adr/`, `CONFORMANCE.md`, and `research/sources.yaml`.
