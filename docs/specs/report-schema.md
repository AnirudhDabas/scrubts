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

Schema 0.2 fixes these fields as `schema_version`, `tool`, `artifact`,
`findings`, `limitations`, and `assumptions`. `artifact.path` is display context
and contains only the input file name in ordinary CLI reports, not the input's
absolute path.

Every finding contains `mechanism`, `status`, `trace`, `evidence`,
`limitations`, and `assumptions`. The typed trace records:

- observation kind;
- verifier identity, version, and availability;
- mechanism, implementation, and detector authority classes;
- authority source IDs and any related-reference boundary;
- configuration identity where meaningful;
- typed `supports` and `does_not_support` inference IDs;
- a stable reproduction command template.

The scanner/provider report construction path assigns this trace. Human,
explain, and JSON renderers do not derive or upgrade authority.

## Determinism

- Findings use stable ordering.
- Map/object serialization must not depend on hash iteration order.
- File traversal order is stable when directory support is added.
- Wall-clock timestamps and ephemeral run IDs must not make canonical evidence
  snapshots nondeterministic. If included, keep them in explicitly noncanonical
  run metadata.
- Human formatting may evolve independently from the versioned JSON contract.

`Report::canonical_report_bytes` is the explicit semantic identity boundary.
It excludes display paths and uses declaration-ordered Serde structs plus sorted
collections. It contains no floating-point values, wall-clock time, host name,
temporary path, terminal capability, or random identifier. This is scrub's
project-specific deterministic JSON contract. It is not RFC 8785/JCS, and local
repeatability does not establish cross-platform determinism.

## Stdout/stderr contract

When `--json` is selected, stdout contains only the report JSON. Diagnostics,
progress, and errors not represented in the report belong on stderr.

Machine JSON is not a human terminal-safety projection. JSON escaping preserves
standards-valid syntax, but Unicode bidi formatting characters may remain in
string values. A consumer that prints untrusted fields must visibly escape
terminal/layout/bidi controls. Human scrub output applies that contract before
rendering.

`--json --explain` emits the same structured schema 0.2 report because the
proof trace is always part of the authoritative report. It does not add a prose
explanation field.

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

The Draft 2020-12 consumer contract is
`schemas/report-0.2.schema.json`. Schema 0.1 reports are not silently decoded as
0.2.
