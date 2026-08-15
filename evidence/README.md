# Evidence index

This index points to the repository's durable proof surfaces. A passing command
establishes only its stated scope; it does not turn `UNKNOWN` into `ABSENT` or
extend a reference implementation into provider authority.

| Evidence family | Source and reproduction | Establishes | Does not establish |
| --- | --- | --- | --- |
| Artifact identity | [`artifact.identity.sha256`](claims.json); `just prove` | SHA-256 and byte length for the exact bytes read | provenance or authorship |
| Unicode conformance | [`CONFORMANCE.md`](../CONFORMANCE.md); `cargo test --offline -p scrub --test unicode_normalization_conformance` | Unicode 17.0.0 behavior over the frozen complete oracle obligations | every future Unicode version or Unicode security judgment |
| Streaming invariance | [`streaming.partition_invariance`](claims.json); `cargo test --offline -p scrub --test streaming_partition` | canonical semantic equality across the specified legal read partitions | all possible I/O failures or absence of bugs |
| Terminal safety | [`terminal.output_safety`](claims.json); `cargo test --offline -p scrub --test terminal_output_safety` | visible escaping for the tested untrusted human-output controls | that raw machine JSON is safe to print unreviewed |
| C2PA corpus replay | [`c2pa-replay-manifest.json`](c2pa-replay-manifest.json); `python tools/c2pa_replay.py --check` | pinned corpus identities and scrub's separate parse, validation, binding, and trust states | independent cryptography or full C2PA 2.4 conformance |
| Public SynthID parity | [`synthid.public_reference_parity`](claims.json); `just prove` | parity with the pinned public CPU reference fixture | Claude or Gemini deployment parity |
| Anthropic provider boundary | [`anthropic.detector_status`](claims.json); `just prove` | correct preservation of `UNKNOWN` for the checked authority snapshot | a negative provider result or current undisclosed service behavior |
| Cross-platform determinism | [`determinism-run-31887807602.json`](determinism-run-31887807602.json); [`run 31887807602`](https://github.com/AnirudhDabas/scrubts/actions/runs/31887807602) | historical equality on three OS families for four frozen fixtures at `a47994e...` | current-HEAD or arbitrary-artifact parity |
| Release preflight | [`release integrity`](../docs/specs/mega-c-release-integrity.md); `just release-check` | fail-closed package and assembly contracts plus the documented historical preflight boundary | a published release, tag attestation, immutability, or platform signing |
| WaterLARP methodology | [`waterlarp.md`](../docs/specs/waterlarp.md); `just waterlarp-check` | authority-aware experiment, calibration, evidence, and aggregation contracts | powered benchmark, provider, authorship, or paper-scale results |

The canonical machine-readable public claim ledger is [`claims.json`](claims.json).
`just prove` executes all 16 rows and writes the current run artifact under
ignored `target/proof/`.
