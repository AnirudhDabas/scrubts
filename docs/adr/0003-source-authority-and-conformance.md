# ADR 0003: source authority and conformance

**Status:** accepted

## Decision

scrub.ts follows `docs/SOURCE_AUTHORITY.md`. Every advertised supported mechanism records its governing source/version, fixture/conformance state, and known deviations in `CONFORMANCE.md`.

## Why

The project implements behavior owned by external standards, vendors, and research methods. Without explicit precedence, summaries/heuristics can silently become “truth.”

## Consequence

Disagreement is surfaced as a deviation/unknown state, not hidden behind a convenience implementation.
