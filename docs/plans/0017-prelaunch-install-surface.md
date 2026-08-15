# Prelaunch install surface

## Goal

Establish that the existing `scrub` package installs from source and that the
installed executable exposes concise, successful root and `inspect` help while
preserving the existing inspection and report semantics.

## Non-goals

- Package-manager installers, publication, releases, tags, networking, update
  checks, telemetry, or release-pipeline changes.
- Argument-parser frameworks, new dependencies, new commands, or broad parser
  restructuring.
- Report, schema, detector, evidence, status, C2PA, provider-watermark,
  WaterLARP, claim-ledger, proof-claim, or research changes.
- Changes to the in-progress launch README milestone.

## Sources / authority

`AGENTS.md`, `docs/PLANS.md`, `docs/specs/v0.1.md`,
`docs/specs/product-proof.md`, `docs/specs/report-schema.md`, and the package
version/source boundary in `docs/specs/mega-c-release-integrity.md` govern this
work. `crates/scrub/Cargo.toml` remains the canonical package-version source.

## Current state

At `8a243b4aa618794dd2235e3dd5e12b140ba8f558`, before this milestone's code
changes, `cargo install --path crates/scrub --locked --root <isolated-root>`
succeeds and installs `bin/scrub.exe`. The installed executable's default,
explain, and JSON controlled-fixture outputs are byte-identical to the
corresponding `cargo run --locked --quiet -p scrub --` invocations. The JSON is
10,452 bytes with SHA-256
`2c88c719b32985af3f1ab2fc01350d5aacc33bf22e87c4f53889cd91c2c3cf07`.

Root `--help` and package-derived `--version` already succeed. `inspect --help`
is currently parsed as an unknown option and exits 2. The package already has
the binary identity and metadata required by this source-install milestone.

## Design

1. Keep the existing hand-rolled parser and one-line diagnostic usage.
2. Add static, concise root and inspect help text that lists only the existing
   command and options and shows the three supported launch invocations.
3. Recognize only the exact `inspect --help` request as successful help before
   inspection parsing. All malformed combinations continue through the strict
   parser and remain failures.
4. Keep `--version` derived from `CARGO_PKG_VERSION`; do not add another version
   source or change package metadata.

## Acceptance criteria

- A fresh isolated source install produces an executable runnable directly as
  `scrub inspect <artifact>`.
- `scrub --help` and `scrub inspect --help` exit 0, use stdout only, and
  accurately describe the existing command and options.
- `scrub --version` exits 0 and prints the Cargo package version.
- Missing paths, unknown commands/options, duplicate options, and excess paths
  remain failures; unknown options are not accepted as paths without `--`.
- Installed default, explain, and JSON output remains byte-identical to the
  Cargo-run route. `--json` and `--json --explain` remain byte-identical.
- Controlled-fixture JSON remains 10,452 bytes with the established SHA-256,
  and Claude remains `UNKNOWN` separately from Unicode U+200B `PRESENT`.

## Implementation steps

1. Add focused root/inspect help constants and the exact inspect-help dispatch.
2. Update command-level CLI tests for both help surfaces, package-derived
   version output, and strict malformed help behavior.
3. Run focused tests, repository gates, a fresh isolated installation smoke,
   and byte-level installed-vs-Cargo comparisons.
4. Audit the final diff and worktree without staging or history operations.

## Validation

Run:

```text
cargo test -p scrub --test cli
cargo test -p scrub --test terminal_output_safety
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
just check
just prove
git diff --check
cargo install --path crates/scrub --locked --root <fresh-isolated-root>
```

Then invoke the installed binary for both help surfaces, version, and all three
controlled-fixture projections; compare their bytes with Cargo-run output and
verify the established JSON digest and JSON/explain invariant.

## Risks / open questions

- Help dispatch must not make `inspect --help <path>` or other malformed
  combinations succeed.
- Installation can require crates.io index access when Cargo's local cache is
  incomplete; this does not add runtime networking to `scrub`.

## Outcome

The existing package required no manifest or packaging changes: isolated source
installation succeeded both before and after implementation. Root help now
lists the one real command, package-derived version surface, and three launch
invocations. Exact `inspect --help` is a successful, concise option reference;
malformed help combinations remain usage errors. Focused tests cover both help
surfaces, package-derived version output, strict malformed invocation behavior,
existing JSON semantics, and terminal safety.

The final fresh install produced
`target/prelaunch-final-install-e72d4178784e4cb19011de19f52c5166/bin/scrub.exe`.
Its controlled-fixture default, explain, JSON, and JSON-plus-explain outputs were
byte-identical to the corresponding Cargo-run outputs. JSON and
JSON-plus-explain were also byte-identical to each other: 10,452 bytes with
SHA-256
`2c88c719b32985af3f1ab2fc01350d5aacc33bf22e87c4f53889cd91c2c3cf07`.
Unicode U+200B remained `PRESENT` at byte/scalar offset 4 while the separate
Claude embedded-text-watermark finding remained `UNKNOWN` with the existing
detector-unavailable and non-parity boundaries.

Focused CLI, binary, and terminal-safety tests passed. Final-state
`cargo fmt --check`, warning-denied workspace Clippy, `cargo test --workspace`,
`just check`, and `just prove` passed; proof completed all 16 claim rows with
`PROOF_COMPLETE`. No report/schema/detector, Cargo manifest, README, research,
release machinery, staging, or Git history was changed.
