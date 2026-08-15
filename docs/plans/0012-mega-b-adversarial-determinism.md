# Mega B: adversarial and determinism evidence

## Goal

Actively challenge scrub-owned evidence boundaries, then make the surviving
streaming, human-output, external-corpus, and deterministic-report invariants
machine-verifiable. Mega B follows committed Mega A at
`99ff0b0b0b2a39f231977d7a1b2d9256c0d562bc`; Mega A and the WaterLARP
methodology remain frozen.

## Non-goals

- New watermark algorithms, provider detectors, GPU work, WaterLARP changes,
  release packaging, attestations, benchmarks, launch material, or a broad C2PA
  implementation.
- A claim that bounded fuzz smoke proves absence of bugs or that selected C2PA
  attack inputs establish general attack resistance.
- A cross-platform equality claim before one workflow run compares actual
  Windows, Linux, and macOS results.
- Git index or history changes.

## Sources / authority

- `rust-fuzz/cargo-fuzz` commit
  `bf2fc668dafda5295aa6fd01825ee67b885f0f2b`, release 0.13.2,
  `Cargo.toml` and `README.md`, MIT OR Apache-2.0. It requires nightly,
  libFuzzer sanitizer support, and a Unix-like x86-64/AArch64 host; it does not
  support native Windows.
- Rust Fuzz Book commit
  `18cfe0e68cfee717392552d3511bac08458d2c01`,
  `src/cargo-fuzz.md`, referenced documentation only.
- `c2pa-org/public-testfiles` commit
  `22beccc075707475b038d8789d0136c009e43143`, root `README.md`,
  `legacy/1.4/image/README.md`, and the four already-vendored JPEGs under
  `legacy/1.4/image/jpeg/`, CC BY-SA 4.0. These are C2PA 1.4 corpus assets.
  Their checked byte lengths are 178,709 (good CA), 178,709 (bad data hash),
  178,709 (bad signature), and 656,258 (bad referenced claim); exact paths and
  SHA-256 identities are in `evidence/c2pa-replay-manifest.json`.
- `contentauth/c2pa-attacks` commit
  `4f750daa888d2ff93a1659fc016be584dc43ae5c`, version 0.1.5,
  `README.md`, `attacks/README.md`, `attacks/rendering.attack`,
  `attacks/special_characters.attack`, and
  `attacks/ten_thousand_characters.attack`, MIT OR Apache-2.0. The payload
  files are referenced by exact identity; no upstream executable or fixture is
  copied. The tool generates hostile assets but does not decide whether a
  target passed.
- `github/docs` commit
  `81ade08c26f13325c0cde8a23cd3bfb85bd0778e`, matrix and workflow-artifact
  documentation, CC BY 4.0. Released action revisions are pinned in workflow
  source: checkout v7.0.1 `3d3c42e...`, upload-artifact v7.0.1 `043fb46...`,
  and download-artifact v8.0.1 `3e5f45b...`.

Exact integration identities and fixture SHA-256 values live in
`research/sources.yaml` and the adjacent machine-readable Mega B manifests.
Later-spec c2pa-rs acceptance of the legacy 1.4 corpus is external-corpus
integration evidence, not independent C2PA 2.4 conformance.

## Current state

HEAD is the baseline commit on `main`; the index and tracked worktree are
clean. Pre-existing unrelated untracked files are outside Mega B and remain
untouched. No GitHub Actions workflows exist. The production inspection path
has one 65,536-byte `Read` loop, incremental UTF-8/property observers,
whole-valid-text normalization retention, content-first C2PA buffering, and
`Report::canonical_report_bytes`. Two unit tests cover a few partitions but do
not establish the requested metamorphic envelope. `terminal_safe` escapes C0
and C1 controls but emits bidi formatting controls raw. Four pinned legacy 1.4
public-testfile JPEGs already exist with adjacent attribution. Mega A proof
identity still calls its explicit file set `MEGA_A_SCOPE` and writes under
`target/mega-a`.

## Design

1. Move the existing inspection implementation behind the package library
   boundary without creating a second scanner. Deterministic tests and fuzz
   targets call the same `inspect_reader` used by the CLI. Compare
   `canonical_report_bytes`, not finding counts or display JSON.
2. Define a valid partitioned `Read` helper that never returns `Ok(0)` before
   EOF. Exhaust every split for small hostile fixtures; use fixed seeded
   partitions for large fixtures and explicit tests at 65,535/65,536/65,537.
3. Put the one human-string escaping primitive in `scrub-report` so the CLI,
   regression tests, and fuzz target share it. Visibly escape C0/C1, ESC/ANSI/
   OSC components, line/layout controls, and Unicode bidi formatting controls.
   Machine JSON remains standards-valid machine output and is not advertised
   as safe to print unreviewed to a terminal.
4. Replay the four already-vendored C2PA 1.4 JPEGs into a stable result schema
   that keeps parse, validation, hard binding, and trust separate. A generator
   and regression compare actual reports to the committed result; default
   proof stays offline.
5. Exercise one selected generated c2pa-attacks contract: the exact rendering
   payload's `Back<BS>Space` value is inserted by deterministic equal-length
   mutation of the generated signed PNG title. The separately pinned
   10,000-character source identity informs the generic long-field renderer
   regression without creating a second C2PA asset. Expected contracts are
   explicit semantic states and safe projection, never “attack blocked.”
6. Keep fuzzing in an independent `fuzz` workspace. Targets are streaming
   partition equivalence, untrusted report JSON import/round-trip coherence,
   and the production human escaping primitive. Ubuntu CI installs pinned
   cargo-fuzz with a pinned nightly, builds each target, runs short bounded
   smoke, and uploads crash artifacts only on failure.
7. Freeze a small fixture manifest with byte SHA-256 values. A Rust example
   invokes the production reader and hashes canonical report bytes. A
   three-platform matrix uploads one result each; a separate Python comparator
   validates all platform and input identities before comparing semantic
   digests and emitting `determinism-matrix.json`.
8. Rename the proof source set to `PROOF_SOURCE_SCOPE`, include every Mega B
   proof-bearing file, and retain base revision, staged state, worktree hashes,
   and unrelated-file exclusion. Add only locally executable deterministic
   claims. Fuzz smoke and pending cross-platform equality remain outside
   default proof claims.

## Acceptance criteria

- Every required streaming boundary case has canonical semantic equality using
  the production scanner and legal `Read` behavior.
- No untrusted human field can emit forbidden terminal or bidi controls raw;
  color is not needed and `NO_COLOR` output remains complete.
- Public corpus results name legacy 1.4, preserve all four C2PA semantic
  layers, and never upgrade unknown trust.
- Each selected adversarial input has source, generation identity, expected
  scrub contract, and actual result.
- Three small fuzz targets build in the declared Unix/nightly lane; native
  Windows execution is not required or claimed.
- Determinism CI compares all three actual platform artifacts and compares
  input identities before report identities. Until it runs, equality is
  recorded as not yet established.
- `just prove` identifies the dirty Mega B source scope and stays reasonably
  runnable offline.

## Implementation steps

1. Record source intake and this plan.
2. Expose the existing scanner through the library and add the complete
   streaming metamorphic suite.
3. Centralize and harden human escaping; add hostile filename, evidence,
   metadata-derivative, long-field, and JSON-boundary regressions.
4. Add C2PA corpus/adversarial manifests, schemas, replay generation, and
   deterministic tests.
5. Add the fuzz workspace, seed corpus, manual commands, and bounded Ubuntu
   workflow.
6. Add deterministic fixture/result schemas, generator, comparator, and
   three-OS workflow.
7. Evolve proof identity and add only established local claims.
8. Run focused tests, required full gates, and one bounded Mega B self-review;
   update Outcome with measured results and limitations.

## Validation

Focused commands are recorded next to each new harness. Completion requires
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `just check`, `just prove`, and `git diff --check`,
plus the streaming suite, terminal regressions, C2PA replay, selected attack
contracts, proof/schema validation, local determinism generation/comparison
validation, and local workflow syntax checks possible on Windows.

Native Windows does not support the pinned cargo-fuzz/libFuzzer lane. Local
verification may inspect and compile ordinary code but must not claim a fuzz
run. Local generation is one-platform evidence only and must not be copied into
a three-platform result.

## Risks / open questions

- c2pa-rs remains the cryptographic parser/validator. Fuzzing its whole parser
  is out of scope; scrub-owned classification, buffering, semantic mapping, and
  output projection are the relevant boundaries.
- The selected untimestamped generated PNG cannot provide reproducible
  validation. Its adversarial contract must preserve UNKNOWN rather than
  inventing an invalid/valid decision.
- GitHub-hosted macOS/Windows/Linux results cannot be established locally.
- `cargo-mutants` is not installed. Global installation was not authorized, so
  targeted mutation testing is explicitly deferred and does not block Mega B.

## Outcome

Implemented and verified on the native Windows authoring host. The production
reader is shared by the CLI, deterministic partition suite, digest generator,
and streaming fuzz target. All seven metamorphic tests passed, including every
small hostile split, malformed/multibyte boundaries, the real 65,536-byte
buffer boundary, zero bytes, and fixed-seed 2 MiB partitions. Human-output,
four-file public-corpus replay, and the selected rendering-payload contract
also passed with the semantic states recorded under `evidence/`.

The isolated fuzz workspace resolved from its pinned lockfile and the bounded
Ubuntu/nightly workflow is present; libFuzzer was not executed on native
Windows. Local canonical-digest generation, strict replay/determinism schemas,
three-platform comparator unit tests, and workflow YAML parsing passed. A real
three-OS comparison has not run, so cross-platform semantic equality remains
not yet established. Mutation testing was deferred because `cargo-mutants` was
not installed and no global installation was authorized.

The required `cargo fmt --check`, Clippy with warnings denied, workspace tests,
`just check`, `just prove`, and `git diff --check` gates passed. The final proof
was `PROOF_COMPLETE` and identified the dirty, unstaged
`proof_relevant_project` at baseline HEAD `99ff0b0b0b2a39f231977d7a1b2d9256c0d562bc`.
The focused self-review corrected an overcounted attack-lane description,
tightened the replay schema, made upstream repository/revision identity
explicit on each replay fixture, and restored pre-existing frozen-fixture
`.gitattributes` rules while appending the new determinism rules. Final
proof-path inspection also added the determinism example to the explicit proof
scope. It found no WaterLARP or frozen Mega A semantic change.
