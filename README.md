# scrub.ts

**See what your model left behind.<br>
Know what it proves.**

scrub inspects machine-readable evidence in AI-generated text and files, then
keeps the observation separate from what its verifier and authority actually
permit you to conclude. Invisible Unicode, statistical watermark signal, and
signed provenance can all be real while supporting very different claims.

## A real signal can support the wrong conclusion

Anthropic's [current primary description](https://www.anthropic.com/news/claude-text-watermark),
checked on 2026-08-15, describes its text watermark as statistical and in the
SynthID Text family. It says there are no hidden characters, detection depends
on Anthropic's key, and its provider detection API is still forthcoming while
implementation details are worked out.

Against that authority, scrub's [time-bounded claims audit](research/claude-watermark-claims/README.md)
records individual public claims, including a provider-level Claude inference
based on a fixed invisible-Unicode scan and a "100% Claude detection" assertion
without exposed detector, evaluation, threshold, or result evidence supporting
the number. The audit does not estimate prevalence or classify whole
publishers.

The counterexamples matter just as much: some tools limit themselves correctly
to Unicode cleanup, public SynthID reference implementations are legitimate
under their disclosed configurations, and some tools explicitly refuse to
certify a provider-detector result. scrub is not anti-watermark or
anti-Anthropic. The finding is narrower: **a signal can be real while the
conclusion outruns its authority.**

| **16 claims / 16 executable proof paths** | **4,780,592** | **3 OS / 4 frozen fixtures** |
| --- | --- | --- |
| Machine-readable ledger, fail-closed runner, and `UNKNOWN` allowed to survive | Independent Unicode normalization oracle comparisons | Identical canonical semantic reports on Windows, Linux, and macOS in historical [run 31887807602](https://github.com/AnirudhDabas/scrubts/actions/runs/31887807602) at revision `a47994e...`; not a universal guarantee |

## Quick start

The verified install path is from source. Rust 1.97.1 is pinned in
[`rust-toolchain.toml`](rust-toolchain.toml).

```console
git clone https://github.com/AnirudhDabas/scrubts.git
cd scrubts
cargo install --path crates/scrub --locked

scrub inspect <artifact>
```

The three projections are:

```console
scrub inspect <artifact>
scrub inspect <artifact> --explain
scrub inspect <artifact> --json
```

Default output is the concise reading. `--explain` exposes verifier,
authority, configuration, supported and forbidden inferences, limitations, and
reproduction. `--json` emits only the typed schema 0.2 report on stdout.
Inspection is local, does not modify the artifact, and does not fetch remote
manifests or provider results. No crates.io or public native release is implied
by this source install.

## Controlled falsification example

A text fixture with one zero-width space demonstrates the boundary directly.
These are selected, non-contiguous lines from current default output:

```console
  Unicode
    PRESENT         Default_Ignorable_Code_Point
      evidence          U+200B at byte offset 4, scalar offset 4
  Claude
    UNKNOWN         embedded text watermark
      verifier          anthropic.provider_detector
                        unavailable in checked authority snapshot
      reference         reference.synthid_text
                        related family; not deployment parity
  UNKNOWN != ABSENT / CLEAN.
```

scrub refuses to transform an arbitrary observable Unicode signal into a
provider or authorship conclusion. The fixture is not an Anthropic watermark
simulation; it is a controlled check that Unicode `PRESENT` and Claude
`UNKNOWN` remain separate.

## Evidence, not a verdict

```text
OBSERVATION -> VERIFICATION -> AUTHORITY -> SUPPORTED INFERENCE
                                      \-> UNSUPPORTED INFERENCE
```

Every public claim has a row in [`evidence/claims.json`](evidence/claims.json).
Run the offline proof paths with:

```console
just prove
```

The command executes all required oracles, fails closed, and writes a proof
artifact that binds the base revision plus changed proof-relevant source. A
passing Anthropic boundary oracle means scrub preserved `UNKNOWN`; it is not a
negative provider result. [`evidence/README.md`](evidence/README.md) is the
short map across claims, conformance, replay artifacts, historical CI evidence,
release preflight, and WaterLARP.

## What is under the surface

- **Unicode.** Property membership and NFC/NFKC difference reporting are pinned
  to Unicode 17.0.0. The complete normalization gate independently checks
  official transformations plus assigned and unassigned scalar obligations.
  Findings are inspection-only; normalization is not sanitization, provenance,
  authorship, or watermark evidence.

- **C2PA.** Presence, manifest validation, hard binding, credential trust, and
  authorship are separate states. scrub integrates pinned `c2pa-rs`, replays a
  selected official corpus, isolates ambient network and sidecar behavior, and
  retains known SDK/time-basis limitations as `UNKNOWN` or `UNSUPPORTED`. This
  is integration and semantic-layer evidence, not an independent cryptographic
  implementation or full C2PA 2.4 conformance claim.

- **Hostile input and determinism.** Human output visibly escapes tested
  terminal and bidi controls. Streaming tests compare canonical reports across
  hostile legal read partitions. Three pinned fuzz targets have bounded Linux
  smoke evidence; bounded fuzzing is bug-finding, not proof of bug absence. The
  exact historical three-OS result and limitations are preserved in
  [`evidence/determinism-run-31887807602.json`](evidence/determinism-run-31887807602.json).

- **Release integrity.** The release path constructs and verifies exact
  deterministic archives for Windows x64, Linux x64, macOS Apple Silicon, and
  macOS Intel, then assembles only a complete four-target set. Historical
  [preflight run 31892107877](https://github.com/AnirudhDabas/scrubts/actions/runs/31892107877)
  at revision `24af24a...` passed all four native builds and exact assembly.
  That is not a current-HEAD result, tag attestation, published or immutable
  release, reproducible compiler build, or platform-signing result.

## C2PA is a ladder, not a badge

```text
presence != validity != hard binding != trust != authorship
```

A valid hard binding establishes that an applicable signed claim is bound to
the inspected bytes under C2PA rules. It does not establish that the claim is
true, that a trusted identity signed it, or that a human or AI authored the
content. scrub v0.1 configures no pinned C2PA trust policy. The bounded behavior
is in the [C2PA inspection contract](docs/specs/c2pa-provenance.md).

## WaterLARP

WaterLARP is the separate authority-aware research harness for public KGW and
SynthID Text baselines. It preserves exact generation, calibration, detector,
key, threshold, evidence-length, operation, and source identities. In
particular, it distinguishes a threshold frozen on clean negatives from a
threshold calibrated on separately transformed negatives; those answer
different survivability questions.

Current WaterLARP is methodology and integration evidence. Its CPU pilot has
tiny N and is not benchmark, provider, authorship, Claude, Gemini, SOTA, or
paper-scale evidence. The powered empirical study has not been executed. See
the [research specification](docs/specs/waterlarp.md) and
[harness documentation](waterlarp/README.md).

## Limitations and responsible use

- `UNKNOWN` is not `ABSENT`, `CLEAN`, or evidence of human authorship.
- No individual signal establishes that a person or model authored an artifact.
- Public-reference output applies only to its named mechanism, key,
  configuration, comparator, threshold, and input conditions.
- Detection can weaken on small, constrained, or factual samples. A complete
  rewrite can remove a statistical signal; that transformation outcome is not
  an observation status.
- C2PA absence does not establish human authorship, and C2PA presence does not
  establish validity, trust, truth, or authorship.
- scrub is not a universal AI detector and should not be used to accuse someone
  of AI authorship from weak or unsupported evidence.

Supported scope and deviations live in [CONFORMANCE.md](CONFORMANCE.md).
Source precedence and exact upstream identities live in
[`docs/SOURCE_AUTHORITY.md`](docs/SOURCE_AUTHORITY.md) and
[`research/sources.yaml`](research/sources.yaml).

## Reproduce and contribute

```console
just check
just prove
python tools/verify_claude_watermark_claims.py
python tools/c2pa_replay.py --check
```

`just check` runs formatting, warning-denied Clippy, and workspace tests. More
focused entry points are indexed in the [evidence map](evidence/README.md).
Contributions should follow [CONTRIBUTING.md](CONTRIBUTING.md); security reports
should follow [SECURITY.md](SECURITY.md).

scrub's own code is licensed under [Apache-2.0](LICENSE). External
implementations, data, and fixtures retain their own terms; see
[third-party notices](THIRD_PARTY_NOTICES.md).
