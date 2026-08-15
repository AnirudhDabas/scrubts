# scrub.ts

**See what your model left behind.<br>
Know what it proves.**

AI models and tools can leave invisible Unicode, statistical watermark signal,
or signed file provenance. Those are different kinds of evidence. `scrub`
inspects a local artifact, keeps each observation attached to its verifier and
authority, and stops where the evidence stops. It does not turn “I found an
invisible character” into “Claude wrote this,” or an unavailable provider
detector into a clean verdict.

This is a contiguous excerpt from the current default output for a
project-created file containing one U+200B ZERO WIDTH SPACE:

```console
Observations

  Unicode
    PRESENT         Default_Ignorable_Code_Point
      evidence          U+200B at byte offset 4, scalar offset 4

  C2PA
    ABSENT          text manifest wrapper

  Claude
    UNKNOWN         embedded text watermark
      verifier          anthropic.provider_detector
                        unavailable in checked authority snapshot
      reference         reference.synthid_text
                        related family; not deployment parity
      supports          mechanism family disclosed; provider detector unavailable
      does not support  Claude watermark presence/absence or provider parity

Interpretation
  A Unicode PRESENT finding supports only its reported Unicode observation.
  It does not establish an Anthropic watermark, Claude involvement, or authorship.
  UNKNOWN != ABSENT / CLEAN.
  No aggregate authorship or artifact-clean verdict is reported.
  Use --explain for the complete evidence and authority chain.
```

The Unicode observation is real. The Claude conclusion is not available. That
difference is the point: **detection is not interpretation**, and
**unknown is not clean**.

## Why this exists

Anthropic's [current primary description](https://www.anthropic.com/news/claude-text-watermark),
checked on 2026-08-15, says future Claude models will generate watermarked text.
It describes a statistical SynthID-Text-family mechanism with no hidden
characters. Provider detection depends on Anthropic's key, and the detection
API remains described as coming “soon” while implementation details are being
worked out. Public “Claude watermark detection” claims already exist, so scrub
audited a bounded convenience sample to separate observations from
provider-level inference.

## Quick start

Install `scrub` from source. The repository pins Rust 1.97.1 in
[`rust-toolchain.toml`](rust-toolchain.toml).

```console
git clone https://github.com/AnirudhDabas/scrubts.git
cd scrubts
cargo install --path crates/scrub --locked

scrub inspect <artifact>
```

Once installed, the three report projections are:

```console
scrub inspect <artifact>
scrub inspect <artifact> --explain
scrub inspect <artifact> --json
```

Default output is the concise reading. `--explain` exposes each observation,
verifier, authority, configuration, supported inference, forbidden inference,
limitation, and reproduction template. `--json` emits the typed schema 0.2
report without human headings or diagnostics on stdout. Inspection is local,
does not modify the artifact, and does not fetch remote manifests or provider
results. `scrub --help`, `scrub inspect --help`, and `scrub --version` describe
the installed command. See the [product contract](docs/specs/product-proof.md)
for the exact projection rules.

## Claude watermark claims audit

scrub's [audit](research/claude-watermark-claims/README.md) asks what a
**bounded, time-bounded convenience sample** actually observes and what
authority its conclusions require. It is not a web census or prevalence
estimate. Its seven claim records cover six public pages, tools, and
repositories. At capture time, the sample included:

- a provider-level, Claude-identification inference built from a fixed
  invisible-Unicode observation;
- a 100% Claude-detection assertion without exposed detector, evaluation,
  threshold, or result evidence supporting that number.

The sample also contains accurately limited Unicode-cleanup claims, legitimate
public SynthID mechanism and reference implementations, and tools that
correctly refuse to claim provider-detector certification. The records classify
individual claims, not whole publishers: a page can state one boundary
correctly and overreach elsewhere.

**Observable signal != provider verification != authorship.**

## The evidence model

```text
OBSERVATION -> VERIFICATION -> AUTHORITY -> SUPPORTED INFERENCE
                                      \-> UNSUPPORTED INFERENCE
```

Finding U+200B supports “this artifact contains a Unicode 17.0.0
`Default_Ignorable_Code_Point`.” It does not support “Anthropic watermark
present” or “Claude authored this.”

A public SynthID implementation can support a result under its disclosed,
pinned configuration. That result does not automatically support Anthropic
deployment parity, a private provider result, or Claude authorship. The closed
status vocabulary and upgrade rules are defined in
[ADR 0001](docs/adr/0001-evidence-status-model.md) and the
[report schema contract](docs/specs/report-schema.md).

## `just prove`

```console
just prove
```

The repository's public claims have executable proof paths. The current
[machine-readable ledger](evidence/claims.json) contains 16 claims; the proof
orchestrator runs every required offline oracle and fails closed if one does not
pass. A passing Anthropic boundary oracle means the repository preserved
`UNKNOWN` correctly—it is not a negative provider result.

The generated proof records the base revision, tested source identity, claim
and gate states, source revisions, fixture identities, and limitations. Read
the [proof contract](docs/specs/product-proof.md#proof-command) before treating
`PROOF_COMPLETE` as broader than the ledger. The command requires the pinned
WaterLARP environment described in its [environment setup](waterlarp/README.md#environment).

## Under the hood

- **Correctness.** A typed semantic `Report` is the source for human, explain,
  and JSON projections. Unicode properties and normalization are pinned to
  Unicode 17.0.0; the normalization gate performs
  [4,780,592 oracle comparisons](CONFORMANCE.md). Streaming tests require the
  same report for every tested legal partition of identical bytes.

- **Provenance.** C2PA carrier presence, manifest validation, hard binding,
  credential trust, and authorship remain separate. scrub integrates pinned
  `c2pa-rs`, replays a selected official C2PA corpus, and keeps a selected
  adversarial rendering case under an explicit local oracle. These are
  integration and semantic-layer results, not an independent cryptographic
  implementation or full C2PA 2.4 conformance claim.

- **Hostile inputs.** Artifact-controlled terminal controls are rendered
  visibly in human output. The independent fuzz workspace targets streaming
  partition equivalence, report JSON import, and human-output escaping. At
  revision `a47994e334aecb868c5f0b07a2cf97da8b09950f`, all three pinned targets
  completed bounded Linux libFuzzer smoke campaigns without a discovered crash
  in [run 31888143927](https://github.com/AnirudhDabas/scrubts/actions/runs/31888143927).
  This is bounded bug-finding evidence, not proof that bugs are absent. The
  exact envelope is documented in the
  [adversarial and determinism contract](docs/specs/mega-b-adversarial-determinism.md).

- **Reproducibility.** Proof artifacts bind the tested source state instead of
  naming only a clean base commit. At revision
  `a47994e334aecb868c5f0b07a2cf97da8b09950f`,
  [run 31887807602](https://github.com/AnirudhDabas/scrubts/actions/runs/31887807602)
  established identical canonical semantic reports on Windows, Linux, and
  macOS for scrub's frozen four-fixture determinism set. This does not
  generalize to current HEAD, other artifacts, or arbitrary cross-platform
  equivalence; workflow source alone is only the contract.

- **Release integrity.** The release contract defines fail-closed assembly for
  Windows x64, Linux x64, macOS Apple Silicon, and macOS Intel archives, with
  deterministic packaging and distinct checksum, build-attestation, immutable
  release, and platform-signing layers. At revision
  `24af24a72adc632852e5fd2114725b28bd3002f1`,
  [preflight run 31892107877](https://github.com/AnirudhDabas/scrubts/actions/runs/31892107877)
  passed source validation, all four native builds, and exact assembly of four
  archives plus `release-manifest.json` and `SHA256SUMS`; independently
  downloaded archive hashes matched both records. `tag-build-and-attest` and
  `create-draft-release` were correctly skipped. This is historical preflight
  evidence, not a current-HEAD preflight, tag attestation, public or immutable
  release, or platform-signing result. See
  [release integrity](docs/specs/mega-c-release-integrity.md).

- **Research.** Public KGW and SynthID reference parity, calibration, evidence
  length, threshold semantics, and provider/public-reference separation live
  in WaterLARP. The bounded Claude claims audit is a separate claim-authority
  artifact. Neither is promoted into a universal AI detector.

## C2PA is a ladder, not a badge

```text
presence != validity != hard binding != trust != authorship
```

A C2PA store can be present while later assurance layers are invalid,
unknown, unsupported, or not applicable. A valid hard binding establishes that
the applicable signed claim is bound to the inspected bytes under C2PA rules.
It does not establish that the claim is true, that a trusted identity signed
it, or that a human or AI authored the content. scrub v0.1 configures no pinned
C2PA trust policy. The bounded behavior and known SDK limitations are in the
[C2PA inspection contract](docs/specs/c2pa-provenance.md).

## WaterLARP

**WaterLARP: Detector Authority, Calibration, and Survivability in Text
Watermarking** is the separate research methodology behind scrub's statistical
work. It keeps generation, calibration, held-out evaluation, detector evidence,
threshold identity, and source authority in machine-readable artifacts.

Its central operational distinction is:

- `fixed_clean_threshold`: freeze a threshold calibrated on clean negatives,
  then measure whether detection persists after an operation;
- `operation_conditioned_threshold`: calibrate on separately transformed
  negatives, then evaluate independently transformed held-out positives and
  negatives under the same operation contract.

Those answer different questions. “The watermark stopped working at the
original operating point” can differ from “the statistical signal ceased to
exist after accounting for the shifted input distribution.”

Current WaterLARP is **methodology and integration evidence**. Its CPU pilot has
tiny N and is explicitly not benchmark, provider, authorship, Claude, Gemini,
SOTA, or paper-scale evidence. The powered empirical study is future work. See
the [research specification](docs/specs/waterlarp.md) and
[harness documentation](waterlarp/README.md).

## Limitations and responsible use

- `UNKNOWN` is not `ABSENT`, `CLEAN`, or evidence of human authorship.
- No individual signal establishes that a person or model authored an artifact.
- Public-reference detector output is evidence only for its named mechanism,
  key, configuration, comparator, threshold, and input conditions.
- Provider-private detectors cannot be reproduced without the provider's
  authority and required configuration. Related mechanism families do not fill
  that gap.
- Detection can be weaker on small samples and constrained or factual text,
  where fewer eligible choices exist. A complete rewrite in which every word
  is replaced can remove the statistical signal.
- C2PA absence does not establish human authorship; C2PA presence does not
  establish validity, trust, truth, or authorship.
- scrub is not a magic “AI detector” and should not be used to accuse someone
  of AI authorship from weak or unsupported evidence.
- The project is early research and engineering work. Supported scope and known
  deviations are recorded in [CONFORMANCE.md](CONFORMANCE.md).

## What scrub implements and what it relies on

| Surface | scrub's work | External authority or implementation |
| --- | --- | --- |
| Evidence semantics | Typed statuses, verifier/authority traces, supported and forbidden inference, CLI projections, proof ledger | Source hierarchy and cited authorities in [`research/sources.yaml`](research/sources.yaml) |
| Unicode | Streaming observation, exact offsets, normalization comparison, conformance and partition oracles | Unicode 17.0.0 standards/data; pinned `unicode-normalization` crate |
| C2PA | Same-byte inspection boundary, conservative state mapping, offline configuration, corpus replay | C2PA 2.4 and pinned `c2pa-rs` for parsing and cryptographic validation |
| Text-watermark research | WaterLARP manifests, calibration/evaluation contracts, authority separation, reproducible evidence | Pinned public KGW, Google DeepMind SynthID, and Transformers references |
| Claude boundary | Provider slot remains `UNKNOWN` without an authoritative runnable detector; public claims are classified against required authority | Anthropic's published mechanism and inference statements |

scrub does not claim to have invented Unicode scanning, normalization, C2PA,
SynthID, KGW, statistical watermarking, calibration, provenance, or source
authority. Exact revisions, integration modes, licenses, and limitations live in
the [source ledger](research/sources.yaml), [conformance matrix](CONFORMANCE.md),
and [third-party notices](THIRD_PARTY_NOTICES.md).

## Reproduce and develop

```console
just check
just prove
python tools/verify_claude_watermark_claims.py
python tools/c2pa_replay.py --check
```

`just check` runs formatting, warning-denied Clippy, and workspace tests.
`just prove` executes the public claim ledger. The two focused commands verify
the bounded Claude audit and pinned C2PA replay artifacts. Deeper reproduction
contracts are indexed by [product proof](docs/specs/product-proof.md),
[adversarial determinism](docs/specs/mega-b-adversarial-determinism.md),
[WaterLARP methodology](docs/specs/waterlarp.md), and
[release integrity](docs/RELEASE_INTEGRITY.md).

Licensed under [Apache-2.0](LICENSE).
