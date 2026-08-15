# Contributing

Prefer small changes that strengthen a concrete supported mechanism or a
reproducible experiment. Correct evidence semantics matter more than feature
count.

Run the core quality gate before submitting a change:

```console
just check
```

Changes to a public claim, verifier boundary, conformance result, or proof path
must also pass:

```console
just prove
```

For a new mechanism or imported method:

- read `AGENTS.md`, `STYLE.md`, and `docs/SOURCE_AUTHORITY.md`;
- define the authority, decision semantics, supported inference, and failure
  states before advertising support;
- update `research/sources.yaml` with the exact upstream revision, license, and
  integration mode;
- add hostile-input tests and independently specified fixtures; record fixture
  origin, identity, and redistribution terms;
- preserve required LICENSE/NOTICE attribution and explain any new production
  dependency's necessity and maintenance judgment;
- derive published tables and figures from committed or archived
  machine-readable results.

Do not turn a related reference implementation into provider parity, a failed
or unavailable check into `ABSENT`, or a transformation outcome into an
observation status. Include the narrow focused tests for the changed surface as
well as the repository gates above.
