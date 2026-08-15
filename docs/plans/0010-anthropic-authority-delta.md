# Anthropic 2026-08-14 authority delta

## Goal

Represent Anthropic's public disclosure that Claude's text watermark is a
version of the SynthID-Text approach while preserving the separate fact that
WaterLARP cannot run Anthropic's exact deployed detector.

## Non-goals

- Claude detection, parameter inference, key recovery, or provider parity.
- Changes to KGW or public-reference SynthID generation, detection, calibration,
  or parity behavior.
- New experiments or changes to the existing integration-pilot artifacts.
- A generic provider plugin or capability framework.
- Git staging or history operations.

## Sources / authority

Primary authority is Anthropic's 2026-08-14 technical article, "How Claude's
text watermark works," read with Anthropic's Help Center marking article. The
Nature SynthID-Text paper and pinned DeepMind reference repository describe the
public reference family, not Claude's deployed configuration. The EU Code of
Practice supplies policy context only.

## Current state

`anthropic.embedded_text_watermark` uses `UNDOCUMENTED_PROVIDER` for mechanism,
implementation, and detector authority. Detector availability and execution are
represented only by booleans. This no longer captures the newly public
mechanism-family information.

The runnable WaterLARP authorities remain `reference.kgw` and
`reference.synthid_text`. The newest complete local integration pilot contains
336 example records and 247 aggregates and is explicitly
`PILOT_NOT_BENCHMARK_EVIDENCE`.

## Design

Keep the existing coarse authority classes. Add an explicit detector-availability
state and one optional, structured provider-deployment record. For Anthropic,
set mechanism authority to `PUBLIC_MECHANISM_PRIVATE_KEY` and record the public
family, private key, undisclosed deployed configuration, forthcoming detector,
unknown API contract, related public reference, and unestablished provider
parity. Keep exact Claude detection non-runnable.

The public SynthID authority retains its existing identity, reference keys,
configuration, detector, and parity semantics. No public-reference result may
satisfy the Anthropic authority identity.

## Acceptance criteria

- Anthropic is provider-documented at the mechanism-family level.
- Its deployed configuration and API contract remain undisclosed or unknown.
- Its detector is `ANNOUNCED_FORTHCOMING`, not available or runnable.
- Public SynthID remains a related reference family without Claude deployment
  parity.
- Positive or negative public-reference decisions cannot become Claude
  `PRESENT` or `ABSENT`.
- New metadata validates and survives canonical serialization.
- Source claims are primary-sourced and provider-reported claims remain labelled
  as claims to test.
- The existing integration pilot and all nine repaired WaterLARP invariants are
  unchanged.

## Implementation steps

1. Freeze primary-source identities and claim boundaries.
2. Extend the authority record and standalone authority schema.
3. Add semantic authority, source, schema, and pilot-scope regressions.
4. Update only stale authority/research documentation.
5. Run narrow checks, full WaterLARP gates, parity, artifact verification, Rust,
   Unicode, and Git safety checks.

## Validation

Run focused authority/source/schema tests first, then `just waterlarp-check`, the
full parity-enabled WaterLARP suite, offline source and pinned-artifact checks,
the existing fresh pilot's schema/checksum/canonical-rescoring checks, both
independent parity commands, `git diff --check`, Unicode normalization
verification, and complete `just check`.

## Risks / open questions

Anthropic has not published its exact configuration, key, detector statistic,
threshold, false-positive target, API contract, or a provider/reference parity
claim. The next architecture trigger is publication of an authoritative,
usable provider detector or API contract.

## Outcome

Implemented and verified on 2026-08-15 without changing the integration-pilot
artifacts. Anthropic is now provider-documented only at the mechanism-family
level; its exact deployment and provider detector remain undisclosed,
unavailable, and non-runnable. The public SynthID reference authority remains a
distinct identity with no established Claude deployment parity.

Validation passed: 25 focused tests, 71 non-parity WaterLARP tests, 74/74 with
parity enabled, 60 unique source records and eight verified pinned artifacts,
the 336-record/247-aggregate integration-pilot verification, independent KGW
and SynthID parity checks, 116 Rust tests, and 4,780,592 Unicode normalization
comparisons. `git diff --check` and both repository quality gates passed.
