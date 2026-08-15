# WaterLARP schemas

Version 2.0.0 uses canonical UTF-8 JSON with sorted object keys, compact
separators, no non-finite numbers, and one terminal LF. JSONL is one canonical
object plus LF per example. CSV is never canonical.

The manifest schema represents arbitrary-N exact generation, calibration, and
test member arrays with row indices and canonical row hashes. Python validation
additionally enforces uniqueness and cross-split disjointness. The example
schema requires canonical detector evidence and types mixed-document
localization as half-open `TOKEN` coordinates. The aggregate schema requires
held-out count fields when a detection rate is reported.

`experiment_spec_id` binds the scientific specification. `run_id` binds that
ID and the validated checkpoint payload. `artifact_set_id` binds promoted
scientific artifact hashes. See `docs/specs/experiment-manifest.md` for the
complete identity hierarchy.

All `$ref` resolution in tests uses a local Draft 2020-12 registry. Validation
must never depend on network access to the schema `$id` host.
`python -m waterlarp validate-run --run <path>` validates the manifest, every
JSONL example, and every aggregate through that registry and reports exact
object counts.

Large raw generations and local results stay under ignored result directories.
An artifact can be promoted only when its manifest, every example, and every
aggregate validate; checksums verify; canonical detector evidence independently
rescores; source authorities are immutable; and its scope is honestly labelled.
Published tables must derive from these machine-readable objects.

The standalone authority-record schema is version 2.0.0. Its optional
`provider_deployment` object separates a provider-documented mechanism family
from an undisclosed deployment and unavailable exact provider detector. It is
not retroactively injected into historical pilot manifests.
