# Conformance matrix

This file records what scrub.ts claims to implement and the authority used to establish that behavior.

| Mechanism | Authority | Pinned revision/version | Fixture/conformance status | Known deviations | Last checked |
|---|---|---|---|---|---|
| Report evidence states | `docs/adr/0001-evidence-status-model.md` | schema 0.1 | contract and round-trip tests | no scanners implemented | 2026-08-11 |
| Claude statistical watermark | Anthropic generated-content marking docs | docs checked 2026-08-10 | unsupported | public third-party detector details forthcoming | 2026-08-10 |
| Unicode security/confusables | Unicode UTS #39 | pin exact Unicode version when implemented | not implemented | n/a | 2026-08-10 |
| C2PA | C2PA 2.4 + `c2pa-rs` | pin crate/repo revision when integrated | not implemented | n/a | 2026-08-10 |

## Rule

A mechanism may not be advertised as supported until its authority/revision, fixtures, tests, and known deviations are recorded here. “Unsupported” and “unknown” are valid public states.
