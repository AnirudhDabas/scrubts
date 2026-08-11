# ADR 0001: Evidence status model

**Status:** accepted

## Context

AI provenance/watermark tooling often conflates “not detected,” “not supported,”
“could not evaluate,” and “proved absent.” That creates scientifically misleading
outputs, especially for proprietary or calibration-sensitive mechanisms.

## Decision

All forensic observations use this closed status vocabulary:

- **PRESENT** — a supported mechanism produced affirmative evidence under its
  documented conditions.
- **ABSENT** — a supported mechanism completed successfully and its documented
  decision rule supports non-detection/absence within its stated limitations.
- **UNKNOWN** — the question is meaningful, but available evidence or required
  prerequisites are insufficient to decide.
- **UNSUPPORTED** — scrub does not implement the required mechanism/input pair.
- **INVALID** — analysis cannot be trusted because required artifact/detector/
  signature/configuration data is malformed or invalid.
- **NOT_APPLICABLE** — the mechanism does not apply to the artifact.

`REMOVED` is not a forensic status. If scrub later gains transformations, action
results live in a separate audit model containing before/after evidence.

## Consequences

- `UNKNOWN` must never be automatically converted into `ABSENT` for summaries.
- UI/human renderers must preserve the distinction.
- Aggregate “clean” booleans are prohibited unless a future spec defines exactly
  which supported mechanisms were evaluated and what “clean” means.
- New detectors must define which status each failure/prerequisite path produces.
