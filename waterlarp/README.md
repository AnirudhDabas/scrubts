# WaterLARP

WaterLARP is scrub.ts's authority-aware Python research harness for public text
watermark mechanisms. It is separate from the Rust forensic CLI and does not
turn experimental scores into provider or authorship claims.

V1 runs only reference KGW and reference SynthID Text. They are materially
different public mechanism families with pinned parity sources. KGW is not a
Claude detector, and the reference SynthID configuration is not Gemini's
deployed configuration. Anthropic now identifies Claude's text watermark as a
version of the SynthID-Text approach, but does not publish its exact deployed
configuration, key, or provider detector. The Anthropic authority record is
therefore mechanism-family documented and still non-runnable. A WaterLARP
reference SynthID result is never a Claude provider-detector result.

## Scientific contract

`UNKNOWN` is not clean. A score supports only its named detector and exact
configuration. Positive reference detection does not establish Claude
watermark presence, and negative reference detection does not establish Claude
watermark absence or human authorship.

The manifest preserves arbitrary-N exact generation/calibration/test members,
canonical source-row hashes, model/tokenizer/config/source identities, a
validated content-addressed generation checkpoint, and checksummed scientific
artifacts. Every example retains exact token IDs and typed detector evidence
sufficient for independent rescoring without that checkpoint.

Every transformed experiment names one threshold interpretation:

- `fixed_clean_threshold` freezes a task/config/evidence-specific clean
  threshold and measures operational persistence;
- `operation_conditioned_threshold` transforms calibration negatives to select
  a shifted-domain threshold, then evaluates independently transformed held-out
  positive and negative test examples under the same operation contract.

Comparators are serialized. KGW preserves the reference strict `>` decision;
SynthID Weighted Mean uses an explicitly WaterLARP-defined inclusive `>=`
decision because its reference score defines no tie classifier. Calibration
records actual exceedances. Held-out FPR records FP/N and an exact 95%
Clopper-Pearson interval. Tiny N remains `UNRESOLVED` even at zero observed FP.

Entropy comes from an immutable clone of the actual base-model next-token logits
before any processor receives a separate mutable clone. Generated token length,
observed post-edit token length, and detector scored-unit length are distinct.
Threshold lookup uses only exact observable scored-unit support. Mixed-document
window maxima are task-specifically calibrated as a full search procedure and
evaluated on held-out pure-negative documents with typed `TOKEN` localization.

The full normative contract is in `docs/specs/waterlarp.md`.

## Environment

The third-party lock intentionally contains no local checkout path. Install it,
then install the local project separately; manifests bind project Git/diff
identity.

```console
python -m venv .venv
python -m pip --python .venv install -r requirements-lock.txt
python -m pip --python .venv install --no-deps -e .
```

## Commands

```console
python -m waterlarp doctor
python -m waterlarp sources verify
python -m waterlarp parity kgw --checkout <pinned-checkout>
python -m waterlarp parity synthid
python -m waterlarp run --config configs/pilot/cpu.yaml
python -m waterlarp aggregate --run results/local/<run-id>
python -m waterlarp validate-run --run results/local/<run-id>
python -m waterlarp verify-run --run results/local/<run-id>
```

Retrieval is an explicit immutable-revision cache phase. Source verification,
aggregation, schema validation, checksum verification, and canonical rescoring
are offline. Raw datasets, model caches, and local results remain ignored.

## Execution scopes and limitations

The SmolLM2-135M CPU integration pilot proves pathways only. Its tiny N cannot
resolve 1% FPR and supports no benchmark, provider, authorship, Claude, Gemini,
or paper-scale conclusion. A future launch-evidence run needs an approved power
plan and targeted review. The paper configuration is explicitly `NOT_EXECUTED`.

Bayesian SynthID, semantic rewrites, adaptive stealing, smoothing attacks, full
WinMax, SPMG, paid judges, and a website remain future work. Generated MBPP code
is not executed without a real hostile-code sandbox. CPU KGW parity does not
establish CUDA RNG parity.
