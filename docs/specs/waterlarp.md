# WaterLARP v1 research specification

## Boundary

WaterLARP is an authority-aware scientific measurement harness. It is a
separate Python package, not a detector in the Rust `scrub` CLI, an attack zoo,
or a provider-authorship detector. It measures named public watermark
mechanisms and preserves `unknown != clean` and
`presence != validity != binding != trust != authorship`.

Only reference KGW and reference SynthID Text are runnable in v1. KGW is not
Claude's watermark. Anthropic identifies Claude's mechanism family as a version
of the SynthID-Text approach, but the reference SynthID configuration is neither
Claude's provider detector nor Gemini's deployed configuration. A positive or
negative decision from either reference detector cannot establish a Claude
provider result, human authorship, or general AI/non-AI origin.

## Authority and runnable mechanisms

Every adapter records mechanism, implementation, detector, key, threshold, and
source authority; model-logit/training prerequisites; guarantee class; and
limitations. `anthropic.embedded_text_watermark` is provider-documented at the
mechanism-family level as a version of the SynthID-Text approach. Its structured
provider-deployment metadata separately records an undisclosed exact
configuration, private provider key, announced/forthcoming detector, unknown API
contract, non-runnable exact detector, and unestablished parity with
`reference.synthid_text`.

Authority identity is exact. A result whose authority is
`reference.synthid_text` cannot satisfy a request for
`anthropic.embedded_text_watermark`, regardless of the reference decision. Until
an authoritative supported provider detector is available and actually runs,
exact Claude watermark status remains `UNKNOWN`; family disclosure cannot turn
it into `PRESENT` or `ABSENT`.

KGW uses the pinned author implementation, gamma 0.25, delta 2.0, context width
4, selfhash/anchored-minhash seeding, a WaterLARP key, and unique repeated-ngram
handling. Its reference decision comparator is strict `z > threshold`.
SynthID uses official Transformers generation and the DeepMind Weighted Mean
score. DeepMind does not define a tie decision for that score; WaterLARP freezes
an explicitly labelled benchmark comparator of `score >= threshold`. Bayesian
SynthID is not enabled.

## Exact split and experiment identity

Each task stores arbitrary-N arrays of exact generation, calibration, and test
members. Member IDs are unique within and across splits. Dataset revision,
source row index, canonical cached-row hash, and prompt-template hash travel
with every selected member. The test split cannot select thresholds or other
hyperparameters.

The experiment specification content-binds those sample sets; model/tokenizer
revisions and artifact hashes; generation settings; scheme/detector/key
configuration; calibration/comparator/evidence-length policy; transforms;
composition/search specification; source authorities; environment lock; seed;
and Git commit/diff identity. The execution ID also binds the validated
generation-checkpoint payload. Promoted checkpoint/examples/aggregate files are
bound by an artifact-set identity and checksums. See
`docs/specs/experiment-manifest.md`.

## Immutable pre-watermark entropy

The custom autoregressive loop obtains the actual next-token base distribution
from `model(...).logits[:, -1, :]` at the current conditioning context. It
immediately detaches and clones that tensor. Shannon entropy and the
EntroBench-compatible spike quantity
`sum_k p_k / (1 + tanh(1) * p_k)` are computed from this preserved snapshot.
Every KGW, SynthID, or other downstream logits processor receives another clone
and may mutate or replace only that copy. Field naming in a Transformers output
is not treated as evidence of ordering; the explicit code path and per-token
base-logit hashes are canonical checkpoint evidence.

## Calibration and held-out false positives

Every threshold identity includes scheme/mechanism and detector config, model,
tokenizer, task/domain, key policy, threshold semantics, comparator authority,
observable evidence length, clean or conditioned regime, operation/strength,
and window-search specification when applicable. Task pooling is forbidden
unless a future specification explicitly defines and labels a pooled
experiment.

Calibration records the real negative N, comparator exceedance count, empirical
exceedance, target FPR, and resolution status. The helper is deterministic and
conservative at ties. Calibration exceedance is not held-out FPR.

Primary negatives are unwatermarked outputs from the same model, task,
tokenizer, decoding policy, and evidence treatment. Held-out test records report
FP, negative N, empirical FPR, and an exact two-sided 95% Clopper-Pearson
interval. When `N * target_fpr < 1`, status is `UNRESOLVED`; zero observed false
positives is not a claim of 0% FPR.

Two threshold semantics remain separate:

1. `fixed_clean_threshold` calibrates on clean calibration negatives and
   measures operational persistence after an edit.
2. `operation_conditioned_threshold` transforms calibration negatives to
   select a shifted-domain threshold, then applies it to independently
   transformed held-out positive and negative test examples under the same
   task/model/tokenizer/operation/strength/evidence/detector/key/search policy.

Transformed test negatives never select the conditioned threshold.

## Observable detector evidence length

Generated token length is the requested sampled prefix. Observed token length
is the post-operation detector-input length. Detector evidence length is what
the detector actually scores:

- KGW: valid unique self-salted n-gram units after repeated-context handling;
- SynthID: valid unmasked scored n-gram positions after repetition masking.

Operational threshold lookup receives only detector evidence length, never the
original nominal length. Exact calibrated evidence counts select thresholds.
Below the smallest calibrated count is `UNSUPPORTED`; an unseen count between
calibrated counts or above support is `UNRESOLVED`. V1 does not interpolate.
Zero valid units remains canonical evidence with an unsupported decision.

## Canonical detector evidence

Every scored example retains exact detector input token IDs, tokenizer and
detector-config identity, exact detector metadata, key provenance, statistic
name, raw score and p-value where defined, scored-unit count, raw detector
evidence, threshold request/selection, threshold value/ID, comparator, decision,
and decision status. KGW evidence includes green count, gamma, z, p, seeding,
and repeated-ngram policy. SynthID evidence includes unmasked count, mask
semantics/version, Weighted Mean, and exact key/config reference. Window records
also retain the full search specification and best span. These checksummed
records can be rescored without the generation checkpoint.

## Transformations and quality

The integration pilot executes seeded token deletion as a controlled,
nonadaptive, key-unaware operation. It does not integrate WaterPark or smoothing
attacks. Evidence no longer exceeding a threshold is not renamed “watermark
removed.” GSM8K reports observable answer preservation and literal/edit damage
metrics are retained. Generated MBPP code is not executed: execution-based code
quality remains unsupported until a real hostile-code sandbox is approved.

## Mixed documents

Composition retains exact half-open `TOKEN` source segments for contiguous and
separated layouts. Whole-document and maximum-over-fixed-windows evaluations are
distinct procedures. Window calibration runs the identical tokenizer, document
length, valid-window filtering, window size, stride, detector evidence policy,
and maximum operation on pure-negative calibration documents. Independent
pure-negative test documents then provide document-level FP/N/FPR/exact CI.

Positive mixed documents retain the predicted best/search span, exact marked
span union, token overlap, union IoU, and defined start/end offset errors. Byte,
Unicode-scalar, and character offsets are unsupported and rejected in v1. Full
WinMax remains future work.

## Execution scopes

The CPU integration pilot uses SmolLM2-135M to prove wiring on the observed
host. Its tiny N cannot resolve 1% FPR and supports no benchmark, provider,
authorship, Gemini, Claude, or paper-scale claim.

A future launch-evidence run is an N-sample execution powered and independently
reviewed for public structural evidence. It has not been executed. The paper
run is the larger preregistered multi-model/task/seed/operation plan needed for
quantitative benchmark claims; its configuration remains `NOT_EXECUTED`.

## Deferred scope

V1 does not add Claude detection, Bayesian SynthID, Unigram, SWEET, X-SIR,
WaterPark attacks, smoothing attacks, paraphrase/translation models, semantic
judges, adaptive stealing, full WinMax, release CI, a website, or marketing
graphics. CPU KGW parity does not establish CUDA RNG parity.
