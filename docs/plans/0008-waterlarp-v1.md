# Milestone 6: first complete WaterLARP research layer

## Goal

Create a Python-only, reproducible research layer that can generate, detect,
calibrate, transform, compose, and aggregate controlled reference KGW and
SynthID Text experiments without changing the Rust forensic runtime or implying
provider deployment semantics.

## Non-goals

- Claude watermark detection or reverse engineering.
- Treating reference KGW or SynthID configurations as provider configurations.
- Paper-scale execution on the local CPU-only machine.
- Adaptive stealing, smoothing, full WinMax, paid judges, a website, or broad
  algorithm coverage.
- Any Git staging or history operation.

## Sources / authority

The governing order is `docs/SOURCE_AUTHORITY.md`. The source identities frozen
in `research/sources.yaml` and `waterlarp/schemas/authority-record.schema.json`
are authoritative for this milestone. In particular:

- KGW papers plus `jwkirchenbauer/lm-watermarking` at commit
  `82922516930c02f8aa322765defdb5863d07a00e` govern KGW behavior.
- the Nature 2024 SynthID Text paper, DeepMind reference commit
  `addb4a158143c7c6851a1308f78b89fceed59683`, and official Transformers
  v5.15.0 commit `5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9` govern SynthID behavior in
  that order;
- Anthropic's retrieved 2026-08-13 help article documents claims and
  limitations only. It still says detector details are forthcoming.
- benchmark projects govern methodology only where the original scheme sources
  do not govern mechanism behavior.

## Current state

HEAD `088862e336fe0f0433a5460af71cbcc4d7f3386c` contains the completed C2PA
milestone. No WaterLARP package exists. The index is empty. Several historical
seed/release files are untracked and are outside this milestone. The host has
Python 3.13.5, PyTorch 2.10.0 CPU, an AMD Ryzen 5 5500U, no CUDA GPU, and only
about 3 GiB available RAM during pre-flight.

## Design

WaterLARP is a separately installable package under `waterlarp/`. Its core
contracts and scientific metrics use ordinary typed Python and NumPy/SciPy.
Model and dataset integrations are explicit optional execution boundaries.
Adapters receive model/tokenizer objects or pinned checkout paths; they do not
use global model state. Canonical JSON uses UTF-8, sorted keys, compact
separators, finite JSON numbers, and a terminal newline for file artifacts.
Run IDs hash the canonical experiment specification, excluding run-time output
and random UUIDs.

The experiment ontology distinguishes fixed-clean and operation-conditioned
thresholds, evasion and spoofing goals, empirical FPR resolution, guarantee
class, authority class, and pilot versus paper-plan execution status.

## Acceptance criteria

- Authority validation refuses the non-runnable Anthropic provider slot.
- KGW and SynthID parity commands verify pinned source identities and literal
  CPU vectors against their authoritative implementations.
- Calibration/test sample IDs are deterministic and disjoint.
- Empirical FPR below sample resolution is `UNRESOLVED`.
- Every transformed experiment names one of the two threshold semantics.
- Token edits, integrity, composition, and window-search calibration are seeded
  and tested.
- Every run emits a canonical manifest, per-example JSONL, and aggregate JSON.
- A real CPU-feasible PILOT exercises both schemes, three task regimes, a token
  edit, mixed authorship, and more than one length, or records a precise
  unexecuted blocker without fabricated output.
- A separate PAPER-PLAN config is frozen and explicitly not executed locally.
- Existing Rust and Unicode gates still pass.

## Implementation steps

1. Freeze source identities, claims, licenses, and related-work notes.
2. Add package metadata, schemas, authority/config/RNG/manifest contracts.
3. Add calibration, confidence, entropy, transform, quality, composition, and
   aggregation primitives with tests.
4. Add explicit KGW and SynthID adapters and pinned parity vectors.
5. Add deterministic dataset descriptors, generation runner, CLI, pilot, and
   paper-plan configs.
6. Run the feasible pilot only after parity and core tests pass.
7. Record exact validation and outcome here.

## Validation

Narrow checks run first, followed by:

```console
python -m pytest
python -m ruff check .
python -m mypy src
python -m waterlarp sources verify
python -m waterlarp parity kgw --checkout <pinned-checkout>
python -m waterlarp parity synthid
just check
powershell -NoProfile -File scripts/verify-unicode-normalization.ps1
git diff --check
```

## Risks / open questions

- CPU RNG and CUDA RNG are distinct in authoritative KGW; CPU parity cannot
  establish CUDA parity.
- SynthID's weighted-mean score requires length-specific empirical/theoretical
  thresholds or the paper's weighted frequentist method. One threshold over
  radically different lengths is prohibited.
- A statistically meaningful 1% empirical FPR result needs materially more
  negatives than the local smoke pilot. Pilot FPR is expected to be
  `UNRESOLVED`.
- Python 3.13 and the current CPU-only environment constrain model/library
  compatibility and pilot scale.

## Outcome

Implemented as an unstaged Python-only research package. Authoritative CPU
parity passed for KGW commit `8292251` and released SynthID reference components
(`addb4a1`, Transformers v5.15.0 `5eddc12`). The real SmolLM2-135M CPU PILOT
`wlrp1-6d1797c7b3005053552abfb1` emitted 96 JSONL records and 84 aggregate
groups across KGW, SynthID, C4, GSM8K, MBPP, nominal lengths 32/64, random
deletion, and contiguous/separated mixed authorship with calibrated window
search. Its empirical FPR is `UNRESOLVED`, as required for the tiny negative
count. Generated MBPP code was not executed because this host lacks a strong
OS sandbox; the opt-in timeout runner is exercised only with trusted tests.

The PAPER-PLAN remains `NOT_EXECUTED`; model, operation-model, and dataset
revisions marked `PIN_BEFORE_EXECUTION` are stop guards, not frozen runnable
identities. A later post-review source check froze exact primary identities for
Watermark under Fire, Sandcastles in the Storm, and Watermark Smoothing Attacks;
all three remain citation-only for this milestone. No implementation or result
depends on them.

Final gates: 60 Python files formatted; Ruff clean; strict mypy clean over 47
source files; all Python tests and both parity tests passed; 58 unique source
IDs and eight pinned artifact hashes verified; all 96 sample and 84 aggregate
pilot objects plus the manifest validated against Draft 2020-12 schemas; and
`just check` passed all 116 Rust tests, including 4,780,592 complete Unicode
normalization cases. No Git staging or history operation was performed.
