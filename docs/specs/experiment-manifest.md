# WaterLARP experiment manifest specification

The Rust report core does not implement this contract. WaterLARP implements it
in `waterlarp/schemas/experiment-manifest.schema.json` and
`waterlarp/src/waterlarp/manifests.py`.

## Identity hierarchy

`experiment_spec_id` is `wlrs1-` plus the first 24 hexadecimal characters of
SHA-256 over canonical `experiment_specification`. The specification binds:

- source-authority identities and exact implementation revisions;
- model and tokenizer revisions and hashes of the artifacts actually loaded;
- dataset revisions, prompt-template hashes, and every selected generation,
  calibration, and test member;
- each selected member's source row index and canonical cached-row hash;
- generation, sampling, scheme, detector, key, calibration, comparator,
  evidence-length, transform, composition, and window-search policies;
- master seed, dependency-lock hash, Git commit, and working-diff hash.

Every task's `sample_sets` entry contains arbitrary-length arrays for
`generation`, `calibration`, and `test`. Member IDs are unique within and
across those arrays. Canonical sample-set SHA-256 therefore changes when any
member, order, row mapping, row content hash, dataset revision, or prompt
template changes. Python validation enforces semantic disjointness that JSON
Schema cannot express across sibling arrays.

`run_id` is `wlrp1-` plus the first 24 hexadecimal characters of SHA-256 over
the experiment-specification ID and validated generation-checkpoint payload
digest. A changed generated token sequence therefore changes execution
identity without changing the scientific specification.

`artifact_set_id` binds the names and SHA-256 values of the promoted generation
checkpoint, canonical example JSONL, and aggregate JSON. `checksums.json` also
binds the manifest. Runtime duration and finalization metadata may vary, but
they cannot alter scientific artifacts while retaining the artifact-set ID.

Authority records use exact mechanism identities. Provider deployments may add
structured metadata for a publicly disclosed family, private key, undisclosed
configuration, detector/API availability, and public-reference relationship.
That relationship is descriptive: `reference.synthid_text` cannot satisfy the
distinct `anthropic.embedded_text_watermark` identity. Historical pilot
manifests are not rewritten when later provider documentation changes.

## Checkpoint and cache contract

The canonical JSON generation checkpoint contains a schema version,
experiment-specification ID, sample-set digest, model identity, tokenizer
identity, generation-config identity, exact prompt/generated token IDs,
per-token pre-processor entropy evidence, and a payload SHA-256. Loading checks
canonical bytes, the payload checksum, all embedded identities, unique entry
identities, and complete expected membership. A stale or tampered checkpoint is
rejected before scoring or finalization.

Cached rows are inputs, not invisible conveniences. Each selected member
records the exact canonical row hash used to construct its prompt. Changing a
bounded row changes the sample-set digest and experiment specification.

## Canonical objects

Canonical JSON is UTF-8, sorted by object key, uses compact separators, permits
only finite numbers, and has one terminal LF for files. JSONL contains one
canonical object and LF per example. CSV is never canonical.

Per-example records are the primitive result. Each contains exact detector
input token IDs, tokenizer/config/key identity, detector raw evidence,
observable scored-unit count, threshold request and selection, comparator, and
decision. Mixed-document records additionally type all locations as half-open
`TOKEN` coordinates. Aggregates are machine-derived from example records and
carry held-out counts and exact intervals; aggregation may not infer missing
provenance.

The local Draft 2020-12 registry resolves all schema references without access
to `scrubts.dev`. Secret key material may be represented by a stable key
reference only when the threat model requires secrecy, but its key policy and
identity must remain bound.
