# Source authority

When sources disagree, scrub.ts uses this precedence unless an ADR documents a deliberate deviation:

1. **formal standard or vendor-published detector/specification** governing the behavior;
2. **pinned official/reference implementation**;
3. **original peer-reviewed/research paper**;
4. **trustworthy independent replication**;
5. **scrub.ts controlled experiment** with frozen config/artifacts;
6. otherwise: report **UNKNOWN** or **UNSUPPORTED**.

A lower-authority source may expose a bug or ambiguity in a higher-authority source. Record the discrepancy; do not silently rewrite the contract.

## Claims

Public technical claims should be classifiable as:

- vendor-reported;
- replicated;
- measured;
- inferred;
- unknown.

## Versioning

External behavior can change. Mechanism support therefore records an exact standard/version/revision and last-check date in `CONFORMANCE.md` and/or `research/sources.yaml`.
