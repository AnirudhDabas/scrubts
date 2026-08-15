# Claude watermark claims audit

This versioned convenience-sample audit asks what six current public
pages/tools/repos claim, what they actually expose, and which authority their
inferences require. It is not a web census, detector benchmark, provider
detector, or removal study.

## Finding

Anthropic currently describes Claude's text mark as a statistical
SynthID-Text-family watermark: nothing is added to the text, there are no hidden
characters, detection uses Anthropic's key, and a detection API will be offered
"soon" while implementation details are still being worked out. In this sample,
Unicode cleanup is real and public SynthID mechanisms are real, but neither is
an Anthropic production-detector result.

<!-- claim-summary:start -->
| Claim | Source | Observable | Required / available authority | Classification |
| --- | --- | --- | --- | --- |
| `claudewatermark-unicode-provider-detection` | `claudewatermark-home` | fixed Unicode list | `provider_gated / public_observation` | `unsupported_provider_inference` |
| `gptcleanup-supported-hidden-unicode` | `gptcleanup-claude-watermark-cleaner` | hidden Unicode cleanup | `public_observation / public_observation` | `accurately_limited` |
| `overchat-claude-detection-100-percent` | `overchat-claude-watermark-article` | unsupported accuracy assertion | `public_verification / none` | `unsupported_provider_inference` |
| `overchat-private-detector-caveat` | `overchat-claude-watermark-article` | public-specification limitation | `provider_gated / public_observation` | `accurately_limited` |
| `google-synthid-open-source-mechanism` | `google-synthid-safeguards` | configurable public mechanism | `public_reference / public_reference` | `mechanism_demo_not_provider_detector` |
| `synthid-text-reference-implementation` | `google-deepmind-synthid-text` | pinned reference implementation | `public_reference / public_reference` | `mechanism_demo_not_provider_detector` |
| `watermarks-remover-vendor-detector-caveat` | `watermarks-remover-repository` | verifiable/best-effort separation | `provider_gated / public_reference` | `accurately_limited` |
<!-- claim-summary:end -->

These are claim-level classifications. A page can contain both a supportable
limitation and a separate unsupported inference.

## Controlled example

The project-created fixture inserts one U+200B (`E2 80 8B`) at zero-based byte
and scalar offset 4 in `This sentence is a controlled project fixture.` Its
SHA-256 is
`e05cdd0954208eb7d75a47571940b2c82a1f3f6b73df7c0a922478a90fc58a83`.
It is not an Anthropic watermark simulation.

Exact machine command:

```text
cargo run --offline --quiet -p scrub -- inspect research/claude-watermark-claims/fixtures/controlled-u200b.txt --json
```

The separately captured explain command appends `--explain` instead of
`--json`. The real explain output includes:

```text
Unicode  PRESENT      Default_Ignorable_Code_Point
             locations=[{"code_point":"U+200B","byte_offset":4,"scalar_offset":4}]
Claude   UNKNOWN      embedded text watermark
             verifier anthropic.provider_detector (unavailable in checked authority snapshot)
             related reference reference.synthid_text (related family; not deployment parity)
```

Raw evidence is in [`evidence/scrub-report.json`](evidence/scrub-report.json),
[`evidence/scrub-explain.txt`](evidence/scrub-explain.txt), and
[`evidence/scrub-run.json`](evidence/scrub-run.json).

## What this establishes

Layer 1, observable: the controlled file contains one Unicode 17.0.0
`Default_Ignorable_Code_Point`, and scrub reports it `PRESENT`.

Layer 2, mechanism/authority: Anthropic describes a statistical
SynthID-Text-family mechanism and provider-specific key. Public SynthID
implementations can verify their own disclosed configurations; they do not
supply Anthropic's production authority.

Layer 3, inference: for this file scrub reports the Anthropic provider slot as
`UNKNOWN`. The supported result is Unicode property presence, not Claude
involvement.

## What this does not establish

The fixture and sample do not establish an Anthropic production watermark,
Claude generation or processing, Claude or human authorship, watermark absence,
successful removal, or prevalence across the web. A watermark, if detected by
appropriate authority, would indicate likely Claude involvement and would not
separate original generation from substantial editing or establish ownership.

## Method

The seven short claim excerpts were captured on
`2026-08-15T15:53:53.8986491Z` from six convenience-sample sources. Live-page
representation hashes and exact repository commits are in `sources.yaml`; full
third-party pages are not redistributed. Anthropic also says small samples and
constrained/factual text can provide less signal, a complete rewrite can remove
the statistical signal, and C2PA file credentials are separate from the text
watermark.

Verify offline with:

```text
python tools/verify_claude_watermark_claims.py
```

## Sources

Primary authority: [Anthropic](https://www.anthropic.com/news/claude-text-watermark).
SynthID authority: [Google AI for Developers](https://ai.google.dev/responsible/docs/safeguards/synthid)
and pinned [`google-deepmind/synthid-text`](https://github.com/google-deepmind/synthid-text/tree/addb4a158143c7c6851a1308f78b89fceed59683).
The remaining sampled pages and exact capture identities are listed in
[`sources.yaml`](sources.yaml).

## Limitations

This is a time-bounded convenience sample, not a prevalence estimate. Static
capture does not reveal undisclosed server-side behavior or private authorized
access. The audit classifies individual captured claims, not whole publishers,
services, or repositories.
