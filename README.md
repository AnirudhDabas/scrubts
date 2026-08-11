# scrub.ts

Evidence-first forensics for AI-generated text, code, and files.

`scrub` reports what it can verify, what it cannot, and why.

> **unknown ≠ clean**

## Status

Early research/engineering development. The initial scope is intentionally narrow:
standards-backed Unicode forensics, C2PA provenance inspection, and a separate
controlled research harness (**WaterLARP**) for public statistical watermark schemes.

Anthropic has announced model-level text watermarking for supported Claude models.
Until scrub.ts has a supported detector with the required prerequisites, Claude
statistical text-watermark status is **UNKNOWN**, not absent.

## Principles

- Evidence before claims.
- Local and inspectable by default.
- Primary specifications and reference implementations over folklore.
- Reproducible experiments over screenshots.
- Calibration and sample-level results over context-free scores.
- Limitations documented beside capabilities.
- No universal AI-detection or watermark-removal claims.

## Planned interface

```console
scrub inspect <path>
scrub inspect <path> --json
scrub doctor
```

Watermark research lives separately in **WaterLARP** so experimental statistical
results cannot silently become production forensic claims.

Start with `docs/SOURCE_AUTHORITY.md`, `docs/specs/v0.1.md`, the ADRs under
`docs/adr/`, `CONFORMANCE.md`, and `research/sources.yaml`.
