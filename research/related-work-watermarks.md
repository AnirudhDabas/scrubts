# WaterLARP related-work matrix

This matrix records what primary papers and project artifacts claim or measure.
It is not an algorithm leaderboard. “NR” means the source did not make that
axis central or WaterLARP has not verified a result suitable for comparison.
Source IDs refer to `research/sources.yaml` where available.

| Work | Goal / schemes | Tasks | FPR semantics | Strength calibration | Entropy / length | Mixed authorship | Attacks | Quality | Authority / reproducibility |
|---|---|---|---|---|---|---|---|---|---|
| KGW + reliability | Distortionary green-list watermark; extended selfhash KGW | Open-ended LM generation and robustness settings | Analytical z/p under null assumptions; repeated n-grams invalidate naïve iid counting | `delta` is a legitimate strength axis; authors recommend gamma 0.25, delta 2.0 baseline | Detectability is entropy- and sample-size-dependent | Not the primary task | Token edits, paraphrase and other reliability analyses | Perplexity and human/quality analysis in papers | Papers and author repository commit `8292251`; WaterLARP literal CPU parity (`kgw`, `kgw-reliability`) |
| SynthID Text | Multi-depth tournament/g-value reference watermark; Mean, Weighted Mean, Bayesian detectors | Gemma/GPT-2 generation and human evaluation | Length-specific empirical/theoretical thresholds or weighted frequentist treatment | Published guarantee class is preserved; WaterLARP does not invent a comparable knob | Detector calibration explicitly depends on token length | NR | Paper robustness transformations | Perplexity and human preference | Nature DOI `10.1038/s41586-024-08025-4`; DeepMind `addb4a1`; Transformers v5.15.0 `5eddc12` (`synthid-text*`) |
| Claude text watermark (provider deployment) | Anthropic reports a version of the SynthID-Text approach; exact configuration and provider detector are not public | Supported Claude text, including Claude-produced translations; code and proofreading discussed | Provider statistic, threshold, calibration, and FPR target are unknown | Provider key is private; WaterLARP keys/configs do not transfer | Short and low-choice text are provider-reported lower-signal cases | Provider-reported dependence on the amount of Claude-selected text | Light edits versus complete rewrite are vendor-reported claims to test | Anthropic reports no practical quality impact; not WaterLARP-replicated | Anthropic technical article (`anthropic-claude-text-watermark`); detector API announced/forthcoming; public SynthID relationship is family-only, not deployment parity |
| MarkLLM | Broad implementation/evaluation toolkit across many algorithms | Multiple generation tasks | Toolkit-specific | Multiple algorithm configs | Some scheme/task analysis | Limited | Broad attack set | Multiple automated metrics | Third-party comparison only, commit `c45ddc4`; never overrides original implementations (`markllm`) |
| WaterBench | Holistic benchmark and comparable watermark strength | Multiple natural-language tasks | Benchmark thresholds | Matched hyperparameter/strength methodology | Length/quality context, not WaterLARP’s full joint surface | NR | Text-level attacks | Task/semantic quality evaluation | ACL benchmark methodology, repo `8f3d779`; not mechanism authority (`waterbench`) |
| EntroBench | Benchmark watermark behavior across model entropy regimes and user operations | Seven tasks spanning C4, translation, MMLU/math/reasoning/code families | Recalibrates thresholds after operations for post-operation separability | Detection-strength matching on calibration data | Three entropy regimes; spike entropy `Σ p/(1+zp)`, `z=tanh(1)` | NR | User operations and edits | Task-dependent quality | ACL 2026 methodology/code `375d406`; WaterLARP names this `operation_conditioned_threshold` and also reports fixed-clean persistence (`entrobench`) |
| UWBench | Evaluate unbiased watermarks, distribution preservation, repeated-query behavior, SPMG, fixed keys | Text generation across model/task settings | Reports stringent FPR operating points | Compares unbiased mechanisms/configs | Detectability/sample effects | NR | Paraphrase and token modification | Distribution and generation quality | Primary paper “Analyzing and Evaluating Unbiased Language Model Watermark,” arXiv:2509.24048. No author-linked repository was found on the retrieved paper page; none is invented (`uwbench`). |
| MarkMyWords | Unified evaluation of watermark quality, size/sample complexity, and tamper resistance | Multiple LM generation tasks | Fixed operating-point evaluation | Compares schemes under quality/size constraints | Watermark size is first-class | NR | Tampering with quality-damage curves | Quality and attack-success tradeoff | Primary project at `wagner-group/MarkMyWords` commit `01c1b8b`; no repository license file found, so citation-only in v1 (`mark-my-words`). |
| WaterSeeker | Detect/localize partial watermarked spans using full text, WinMax, fixed windows, and WaterSeeker | C4 prompts plus inserted generated segments in Wikipedia documents | Targets document-level FPR for search | Includes varying watermark strengths | Segment/document length varies | Central | Composition rather than semantic attack | Localization IoU plus detection | Findings NAACL 2025 paper; repo `01ee6d9` has no license file, so no code is copied (`waterseeker`) |
| WaterPark / Watermark under Fire | Platform/taxonomy for watermark robustness under attack knowledge and capabilities | Multiple models/tasks | Attack-evaluation operating points | Configuration-specific | Varies | Some composition-like settings | Free-text, token, model/query-aware attack taxonomy | Quality constraints alongside robustness | ACL `2025.findings-emnlp.1148`, DOI `10.18653/v1/2025.findings-emnlp.1148`; author-linked WaterPark revision `76b66df` has no repository-wide license, so it is citation-only and no code is adapted (`waterpark-under-fire`). |
| Watermarks in the Sand | Impossibility of strong watermarking for broad generative-model conditions | Theoretical and experimental illustrations | Security-game semantics | Not a benchmark tuning protocol | Sample/quality constraints motivate limits | Related through edits/composition | Quality-preserving removal/impossibility setting | Indistinguishability/quality assumptions | ICML 2024 primary paper (`watermarks-in-sand`) |
| Sandcastles in the Storm | Limits/robustness debate under transformations and realistic utility constraints | Generative text settings | Threat-model dependent | NR | Sample complexity relevant | NR | Adversarial transformations | Utility/damage is central | “Sandcastles in the Storm: Revisiting the (Im)possibility of Strong Watermarking,” ACL `2025.acl-long.1436`, DOI `10.18653/v1/2025.acl-long.1436`; citation-only and no quantitative claim is imported (`sandcastles-in-storm`). |
| Watermark Stealing | Infer watermark behavior for evasion and spoofing/false attribution | LLM text generation | Attack success at detector operating points | Attacker learns from queries/samples | Sample and query budget | NR | Key/mechanism inference, scrubbing and spoofing | Quality constrained | Primary paper and MIT repo (`watermark-stealing`); future v1.1 execution |
| Watermark Smoothing Attacks against Language Models | Black-box/score-informed smoothing attack against token watermarks | Generated text | Detector-dependent | Attack hyperparameters | Length affects confidence | NR | Adaptive smoothing | Quality preservation required | ACL `2025.findings-emnlp.264`, DOI `10.18653/v1/2025.findings-emnlp.264`; author-linked Apache-2.0 repo revision `5acda5f`; citation/future-threat-model only and no attack implementation is integrated (`confidence-smoothing-attack`) |
| SWEET | Entropy-aware watermarking for code generation | Code synthesis | KGW-like statistic under entropy gating | Entropy threshold plus watermark parameters | Token entropy is central | NR | Code transformations | Execution/pass metrics | ACL 2024 paper and author repo; citation/related work only (`sweet`) |
| X-SIR | Cross-lingual consistent semantic-informed watermark | Translation/cross-lingual text | Detector operating point | Scheme parameters | Cross-lingual length/semantics | NR | Translation | Semantic/translation quality | Paper arXiv:2402.14007 and author repo `zwhe99/X-SIR` commit `9543e14`; no license file found, so no code copied (`x-sir`) |
| TextSeal (2026) | Public watermark/radioactivity-oriented research architecture | Broad text/model experiments | Configuration-specific | Supports frontier analysis | Length/sample effects | Potentially | Multiple transformations | Quality and radioactivity metrics | 2026 primary paper/repo, reference only (`textseal`) |

## What WaterLARP combines or changes

WaterLARP does not claim novelty from adding more schemes. Its v1 contribution
is a tightly authoritative joint protocol over two public mechanisms:

1. exact mechanism/implementation/detector/key/threshold authority travels with
   every run, including a non-runnable provider slot whose public family is
   separate from deployment and detector parity;
2. fixed-clean persistence and operation-conditioned separability are reported
   side by side rather than silently substituted;
3. pre-watermark entropy, fixed length buckets, and subgroup FPR are first-class
   aggregation axes;
4. controlled token edits are paired with task-native and literal-integrity
   damage, producing survivability-damage frontiers instead of an isolated
   “attack success” number;
5. mixed authorship is evaluated with whole-document and procedure-calibrated
   fixed-window search, so maximum-over-windows multiple testing is included in
   document FPR;
6. canonical sample records, manifests, source authorities, and aggregates form
   the website/reproduction interface; no number needs manual transcription.

These are protocol integrations and semantic clarifications, not a claim that
prior benchmarks lacked every individual component.
