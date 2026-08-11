# Report schema specification

## Purpose

The report is an evidence record, not a marketing summary. Its serialized form
must remain machine-readable, versioned, deterministic where practical, and
explicit about unsupported/unknown states.

## Finding status

The v0.1 closed status set is:

- `present`
- `absent`
- `unknown`
- `unsupported`
- `invalid`
- `not_applicable`

`removed` is not a finding status. A future transformation subsystem may record
an action/outcome separately from forensic findings.

## Required top-level concepts

A report must represent:

- `schema_version`
- scrub/tool version
- artifact identity (path/display name, byte length, content SHA-256 when read)
- zero or more observations/findings
- mechanism/engine identity per observation
- evidence per nontrivial observation
- limitations/assumptions when required for interpretation

Exact field names may be proposed in Milestone 1, but semantic meaning must not
contradict the ADRs.

## Determinism

- Findings use stable ordering.
- Map/object serialization must not depend on hash iteration order.
- File traversal order is stable when directory support is added.
- Wall-clock timestamps and ephemeral run IDs must not make canonical evidence
  snapshots nondeterministic. If included, keep them in explicitly noncanonical
  run metadata.
- Human formatting may evolve independently from the versioned JSON contract.

## Stdout/stderr contract

When `--json` is selected, stdout contains only the report JSON. Diagnostics,
progress, and errors not represented in the report belong on stderr.

## Statistical detector extension requirements

A future statistical-watermark observation must be able to retain, as
applicable:

- detector implementation and revision;
- model/tokenizer revision;
- detector config/key identifier or non-secret hash;
- statistic;
- threshold;
- calibration procedure / target false-positive rate;
- analyzed token/sample length;
- preprocessing;
- detector prerequisites;
- limitations.

The schema must not force every deterministic scanner to populate these fields.

## Compatibility

Schema changes that alter field meaning require a schema-version change or an
explicit compatibility decision recorded in an ADR.
