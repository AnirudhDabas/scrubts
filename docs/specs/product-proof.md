# Product and proof contract

## Inspect projections

`scrub inspect <artifact>` is the concise human projection of one typed report.
It groups Unicode, C2PA, and provider-watermark evidence, preserves status names,
and states the interpretation boundary. It does not render empty detail sections
or an aggregate clean/authorship verdict.

`scrub inspect <artifact> --explain` renders the same report with each finding's
observation, verifier, authority classes and source IDs, configuration, supported
and forbidden inferences, limitations, and reproduction template. It does not
rerun inspection or infer authority from prose.

`scrub inspect <artifact> --json` emits one schema 0.2 JSON object and a trailing
newline on stdout. Successful JSON output has no headings, ANSI/OSC controls, or
diagnostics. `--json --explain` is intentionally the same structured report.

## Public claim ledger

`evidence/claims.json` is the canonical machine-readable public-claim ledger.
It uses the source-authority claim classes (`vendor_reported`, `replicated`,
`measured`, `inferred`, and `unknown`) and resolves authority IDs against
`research/sources.yaml`. The Draft 2020-12 contract is
`schemas/claims-0.1.schema.json`.

An UNKNOWN claim can have a passing boundary oracle. That PASS means the
repository correctly preserves UNKNOWN; it is not a negative detector result.
Anthropic claims in the offline ledger describe the checked authority snapshot
identified by the repository sources, not live provider state at proof time.

## Proof command

`just prove` reads the claim ledger and executes every listed required oracle
without a shell or network access. A row passes only when its internal oracle or
subprocess succeeds. Any failed row makes the command non-zero and sets the
overall result to `PROOF_FAILED`.

The command writes deterministic-identity data to `target/mega-a/proof.json`,
validated by `schemas/proof-0.1.schema.json`. It includes the base Git revision
and a `tested_source` identity for the relevant Mega A tracked and untracked
source paths. A dirty worktree therefore cannot be mistaken for the base
revision alone; unrelated untracked files and ignored build output are outside
that identity scope.

`target/mega-a-control/proof-state.json` records the current invocation as
`PROOF_RUNNING`, `PROOF_COMPLETE`, or `PROOF_FAILED`. Consumers must require
`PROOF_COMPLETE` before treating the
canonical proof artifact as the result of the current invocation. Setup,
execution, and output failures record FAILED, so a previous successful
`proof.json` cannot masquerade as current success.

The proof includes the Git revision,
claim/gate states, source revisions, committed fixture SHA-256 values, any
locally established report digest, and explicit limitations. It omits execution
timestamps and gate durations. No proof digest is claimed because Mega A does
not define canonical proof bytes.

Default proof establishes only the rows present in the ledger. In particular:

- KGW source and committed fixture identities are checked offline; the pinned
  upstream KGW checkout is not executed;
- pinned public-reference SynthID CPU parity is executed, but that does not
  establish Claude provider parity;
- ignored local WaterLARP pilot results are not prerequisites, and pilot scope
  is not benchmark evidence;
- local report repeatability is not a Windows/Linux/macOS determinism result;
- RFC 8785/JCS compliance is not claimed.
- Offline Anthropic gates establish only the pinned/check source snapshot; a
  fresh official provider check is a separate prelaunch operation.
