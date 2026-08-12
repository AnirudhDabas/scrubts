# Conformance matrix

This file records what scrub.ts claims to implement and the authority used to establish that behavior.

| Mechanism | Authority | Pinned revision/version | Fixture/conformance status | Known deviations | Last checked |
|---|---|---|---|---|---|
| Report evidence states | `docs/adr/0001-evidence-status-model.md` | schema 0.1 | contract and round-trip tests; production DICP finding round trips | no known deviations | 2026-08-11 |
| Unicode `Default_Ignorable_Code_Point` | Unicode UAX #44 and pinned `DerivedCoreProperties.txt` | Unicode 17.0.0; 27 ranges; 4,174 code points; semantic SHA-256 `5d2e0f0aaa2d84955d13925234b7f806a613e25f0ab0fc9666b32b9120a6a42c` | production table exactly matched to the licensed compact fixture oracle; 24-case corpus exercised through the compiled CLI | UTF-8 single-file property observation only; no bidi-control, normalization, confusable, or sanitization analysis | 2026-08-11 |
| Claude embedded text watermark | Anthropic Help Center, "How Claude marks AI-generated content" | documentation verified 2026-08-11 | unsupported; status remains `UNKNOWN` unless a supported detector evaluates the artifact | technical detection mechanisms/details forthcoming; absence of a detected mark does not establish that content was not AI-generated or processed | 2026-08-11 |
| Unicode security/confusables | Unicode UTS #39 | pin exact Unicode version when implemented | not implemented | n/a | 2026-08-10 |
| C2PA | C2PA 2.4 + `c2pa-rs` | pin crate/repo revision when integrated | not implemented | n/a | 2026-08-10 |

## Rule

A mechanism may not be advertised as supported until its authority/revision, fixtures, tests, and known deviations are recorded here. “Unsupported” and “unknown” are valid public states.
