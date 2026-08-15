# scrub.ts

Evidence-first forensics for AI-generated text, code, and files.

`scrub` reports what it can verify, what it cannot, and why.

> **unknown ≠ clean**

## Status

Early research/engineering development. The initial scope is intentionally narrow:
standards-backed Unicode forensics, C2PA provenance inspection, and a separate
controlled research harness (**WaterLARP**) for public statistical watermark schemes.

Anthropic documents embedded, model-level text watermarking for supported
Claude-generated text and identifies the mechanism family as a version of the
SynthID-Text approach. Anthropic has not published its exact deployed
configuration, provider key, or usable detector/API contract. WaterLARP's
public SynthID reference lane is related research evidence, not Claude detector
parity.
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
NFKC-difference observations for one file. It also reports layered C2PA 2.4
carrier/store presence, manifest integrity, hard binding, and credential trust
for Appendix A.8 unstructured text plus embedded PNG, JPEG, and SVG manifests.
Inspection is in-memory and does not read sidecars, fetch remote manifests, use
ambient trust roots, or modify the artifact.

Embedded-store presence is parsed separately from assurance. When released
SDK APIs cannot establish a certificate and revocation validation basis that
is independent of the current clock, manifest validation and hard binding are
reported `UNKNOWN` rather than exposing a result that can change over time.

C2PA provenance is not Claude's embedded text watermark. A valid hard binding
establishes that the applicable signed claim is bound to the inspected content
under C2PA rules; it does not establish authorship, truth, whether AI created
the content, or whether the file has never been edited. Trust is reported
separately, and v0.1 configures no pinned C2PA trust policy.

Invisible Unicode is evidence, not an interpretation. C2PA 2.4 can itself
encode signed provenance inside Unicode Variation Selectors, so scrub reports
the Unicode property and provenance context independently.

Normalization findings are neutral
comparisons against the untouched input; they do not rewrite text or establish
security risk, provenance, authorship, or watermark presence.

`scrub doctor` and broader mechanism families remain planned.

Watermark research lives separately in **WaterLARP** so experimental statistical
results cannot silently become production forensic claims.

Start with `docs/SOURCE_AUTHORITY.md`, `docs/specs/v0.1.md`, the ADRs under
`docs/adr/`, `CONFORMANCE.md`, and `research/sources.yaml`.
