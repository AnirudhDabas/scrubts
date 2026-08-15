# Mega B adversarial and determinism evidence

## Streaming invariant

For the exact same artifact bytes and display classification path,

```text
inspect_reader(maximal Cursor reads).canonical_report_bytes()
==
inspect_reader(any tested legal Read partition).canonical_report_bytes()
```

Both sides call the production `scrub::inspect_reader`; there is no test-only
scanner. A partitioned reader returns positive byte counts until the exact
input is consumed and returns `Ok(0)` only at EOF. The deterministic suite in
`crates/scrub/tests/streaming_partition.rs` exhausts every split of small
hostile fixtures, every byte position in representative 2/3/4-byte UTF-8
scalars, malformed UTF-8 positions, 65,535/65,536/65,537-byte cases, a late
invalid byte, multiple Unicode evidence types crossing the production buffer,
zero bytes, and fixed-seed partitions of a 2 MiB artifact.

Reproduce with:

```text
cargo test --offline -p scrub --test streaming_partition
```

## Human-output contract

Every artifact-controlled string that reaches human output passes through
`scrub_report::human_safe`. C0 and C1 controls, ESC/ANSI/OSC components,
carriage return, line feed, tab, backspace, Unicode line/paragraph separators,
ALM/LRM/RLM, bidi embeddings/overrides, and bidi isolates are represented as
visible lowercase `\u{...}` text. Renderer-owned line feeds remain trusted
layout. Status words and labels remain plain text, so color is never required
and `NO_COLOR` output is complete.

This contract also applies to path and dependency diagnostics. Tests cover the
escaping primitive, complete renderer, malicious display name, actual bidi
filename, OSC 8 shape, long fields, and the selected C2PA metadata derivative.

JSON is different: `--json` remains one standards-valid serialized report and
one trailing line feed, with diagnostics on stderr. JSON strings may contain
Unicode bidi formatting characters because machine JSON is not a terminal
projection. Consumers that print untrusted JSON fields must apply their own
terminal escaping. scrub does not market raw JSON as safe to print unreviewed.

## Targeted fuzzing

The independent `fuzz/` workspace contains three targets:

- `streaming_partition_equivalence` varies bytes and legal partitions, then
  compares canonical semantic bytes or typed error text from the same scanner;
- `report_json_import` tries arbitrary JSON, supplies a structure-aware valid
  artifact-path mutation, checks successful untrusted parse/serialize/parse
  coherence, and requires an ontology-invalid verifier mutation to be rejected;
- `human_output_escape` tests the production escaping primitive for forbidden
  output, idempotence, and preservation of safe text.

The workspace pins `libfuzzer-sys` 0.4.13 in its own lockfile. The Ubuntu smoke
workflow pins cargo-fuzz 0.13.2 and `nightly-2026-08-14`, builds every target,
runs each for at most 20 seconds, and uploads crash/timeout artifacts on
failure. A bounded smoke is bug-finding evidence only.

Longer compatible-host commands are:

```text
cargo +nightly-2026-08-14 fuzz run streaming_partition_equivalence -- -max_total_time=3600
cargo +nightly-2026-08-14 fuzz run report_json_import -- -max_total_time=3600
cargo +nightly-2026-08-14 fuzz run human_output_escape -- -max_total_time=3600
```

cargo-fuzz/libFuzzer does not support native Windows. Mega B does not require
WSL, does not claim a local Windows fuzz run, and keeps fuzz smoke out of
ordinary `just prove`.

## External C2PA corpus replay

The replay uses four already-vendored JPEGs from
`c2pa-org/public-testfiles` commit
`22beccc075707475b038d8789d0136c009e43143`, paths under
`legacy/1.4/image/jpeg/`, licensed CC BY-SA 4.0. The selected corpus identity is
C2PA 1.4. c2pa-rs 0.90.12 accepting or rejecting these files while scrub uses a
C2PA 2.4 semantic contract is later-validator interoperability/integration
evidence. It is not proof that scrub independently implements all of C2PA 2.4.

`evidence/c2pa-corpus-results.json` records source/path/hash/category/license
and separate scrub parse, validation, binding, and trust states. A known-good
valid/bound file remains trust `UNKNOWN` because scrub has no pinned trust
roots. The three bad vectors are never promoted from parsed store presence to
valid, bound, or trusted. c2pa-rs supplies cryptographic parsing and validation;
scrub supplies the same-byte boundary and semantic mapping.

Reproduce both public and adversarial records offline with:

```text
python tools/c2pa_replay.py --check
```

## Selected c2pa-attacks contract

The source is `contentauth/c2pa-attacks` commit
`4f750daa888d2ff93a1659fc016be584dc43ae5c`, MIT OR Apache-2.0. Mega B selects
one exact `attacks/rendering.attack` line, `Back<0x08>Space`. The replay replaces
the unique equal-length `signed-png` title in the pinned generated PNG and
changes no other bytes.

The oracle is: store parse remains `PRESENT`; validation and binding remain
`UNKNOWN` because the fixture lacks the reproducible time basis required to
run phase two; trust is `NOT_APPLICABLE`; and the hostile title/control is not
projected into human output. The mutation invalidates signed bytes, but the
time-basis prerequisite prevents this case from being a signature-rejection
oracle. `evidence/c2pa-adversarial-results.json` records the generated input
digest and actual states. This is one selected scrub-owned contract, not a
claim that an attack was blocked or that scrub is secure against C2PA attacks.

The pinned upstream 10,000-character payload identity informs the renderer's
long-field regression; no large generated C2PA asset is vendored merely to
increase case count.

## Cross-platform semantic determinism

`evidence/determinism-fixtures.json` freezes four fixture IDs, exact byte
SHA-256 values, expected capabilities, and the generation command. Explicit
`.gitattributes` entries disable checkout text conversion for every selected
fixture.

The workflow has two stages:

1. Windows, Linux, and macOS each verify fixture bytes, inspect them through
   the production reader, hash `Report::canonical_report_bytes`, and upload one
   `determinism-platform.json` artifact containing no time, host, or temporary
   path.
2. A separate required job downloads all three artifacts, requires one workflow
   revision and identical fixture ID/input-digest/capability sets, then compares
   semantic report digests. Input mismatch fails before report comparison. The
   resulting `determinism-matrix.json` permits no capability differences.

The local command below verifies the generator on one host only:

```text
cargo run --offline --locked -p scrub --example determinism -- --manifest evidence/determinism-fixtures.json --platform windows --project-revision <git-revision> --output target/mega-b/determinism-local.json
```

Workflow source and a local Windows result do not establish cross-platform
equality. Until a real three-OS comparison artifact reports `ESTABLISHED`,
cross-platform semantic determinism is **NOT YET ESTABLISHED** and has no
default claim-ledger row.

## Proof and mutation boundaries

`just prove` identifies `proof_relevant_project`, preserving the base HEAD and
explicit dirty/staged state while hashing changed proof-relevant production,
test, evidence, schema, tool, fuzz, workflow, and focused documentation files.
Unrelated human-owned files and ignored build/fuzz artifacts are excluded.

The default ledger includes deterministic streaming, terminal, external-corpus,
and selected-adversarial rows. Fuzz execution remains in its bounded CI lane.
Cross-platform equality remains pending. `cargo-mutants` was not installed on
the authoring host, and no global-tool installation was authorized, so targeted
mutation testing is explicitly deferred without blocking Mega B.
