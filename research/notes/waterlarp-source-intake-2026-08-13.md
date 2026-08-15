# WaterLARP source intake, 2026-08-13

## Anthropic launch-catalyst check

The official article “How Claude marks AI-generated content” returned HTTP 200
at `2026-08-13T22:56:23Z`. Its UTF-8 HTML representation was 336,891 bytes with
SHA-256 `6fabc4c3a482e20b9f93e7bb1dc472d062ab1699a28b33ecdab63fb69d6c1df3`.
It still says Anthropic will share detection mechanisms in “forthcoming
technical documentation.” It distinguishes embedded text watermarks from C2PA
file provenance and warns that detection is not full provenance and absence is
not proof content was not AI-generated or processed. No public mechanism or
detector was disclosed. The project stop condition was therefore not triggered.

The post-review catalyst re-fetch at `2026-08-14T05:22:26Z` again returned HTTP
200. The 336,891-byte UTF-8 representation had SHA-256
`419876a6a7605645abe675b6ee4885732e2595155cc7cb0098e6f7670db33dae` and still
said detection details would appear in forthcoming technical documentation. It
published no third-party detector, verifier/API, key/configuration, algorithm,
or mechanism specification. The architecture catalyst was not triggered.

This check was repeated from the same official URL at `2026-08-14T05:22:26Z`.
The 336,891-byte UTF-8 representation had SHA-256
`419876a6a7605645abe675b6ee4885732e2595155cc7cb0098e6f7670db33dae` and
still said, “We’ll share details on detection mechanisms in forthcoming
technical documentation.” No detector, verifier/API, mechanism specification,
or third-party key/configuration was published, so the architecture catalyst
remained inactive.

## KGW

The author repository was pinned at
`82922516930c02f8aa322765defdb5863d07a00e` (2025-09-17 commit date). Its
README still recommends the extended implementation with gamma 0.25, delta
2.0, context width 4, `selfhash` (`anchored_minhash_prf` with self salt), and
`ignore_repeated_ngrams=True`. It explicitly says the demo base key 15485863
should not be reused for deployment and that CPU and CUDA PyTorch generators
differ. WaterLARP derives benchmark key `4182307207024115832` from a documented
research seed.

The repeated-ngram detector unit is each unique `(context,target)` n-gram. For
selfhash, the scored n-gram length is context width because the target is part
of the self-salted context. Literal CPU parity froze greenlists and a 15-unit
score: 4 green units, fraction 0.26666666666666666, z
0.14907119849998599, one-sided normal p 0.4407487260955068. This establishes
parity only for CPU semantics.

## SynthID Text

DeepMind reference commit `addb4a158143c7c6851a1308f78b89fceed59683`
documents Mean, Weighted Mean, and per-key-trained Bayesian detectors. It
recommends length-specific empirical/theoretical thresholds or Appendix A.3.1's
weighted frequentist treatment across varying lengths. Default reference
`ngram_len` is 5 (H=4 context), with context history 1024. Bayesian requires
independent representative training data and training per unique key; it is not
a v1 blocker and was not run.

Generation uses released official Transformers v5.15.0 commit
`5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9`. Its source explicitly says the
reference hashing differs from the Gemini App and trained detectors do not
transfer. WaterLARP labels this a REFERENCE CONFIGURATION and performs
length-specific empirical calibration. Literal parity froze official g-values,
repetition mask, and Weighted Mean 0.5352272727272728 for a five-key CPU vector.

## EntroBench semantics

Pinned code `375d40601826e775b4bd7d790a19563b477bc5b6` computes spike entropy as
`sum_k p_k / (1 + z p_k)` with `z=0.7615941589914151 = tanh(1)`. It evaluates
seven task families across three entropy regimes and user operations with
post-operation thresholding. WaterLARP preserves that separability
interpretation as `operation_conditioned_threshold`; it never substitutes it
for `fixed_clean_threshold` persistence.

## Partial spans and attacks

WaterSeeker constructs mixed documents by inserting generated watermarked spans
into natural text and compares full-text, exhaustive WinMax, and fixed-window
search with localization IoU. Its pinned repository has no license file, so v1
uses paper-level methodology only and implements an independent fixed-window
procedure calibrated on pure negatives.

WaterPark/Watermark Under Fire informs the typed knowledge taxonomy. Watermark
stealing separates evasion from spoofing and is deferred. Watermarks in the
Sand and Sandcastles in the Storm motivate reporting detection survival against
content damage without assuming one side of the robustness debate. SWEET and
X-SIR are related entropy/code and cross-lingual work, not integrated schemes.

MarkMyWords was frozen for citation at project commit
`01c1b8be5d740d7b2cd4fb01a8fb81bdfc2e6a57` and X-SIR at
`9543e14f3497749bc20ac4108e047c164b3537b4`; neither pinned tree exposed a
repository license file, so no code was copied.

The post-review check froze three previously unresolved slots. “Watermark under
Fire: A Robustness Evaluation of LLM Watermarking” is ACL
`2025.findings-emnlp.1148`, DOI
`10.18653/v1/2025.findings-emnlp.1148`, with author-linked WaterPark revision
`76b66dfa604075c9c79be71dcaebb5afe652d882`; that tree has no repository-wide
license, so code remains citation-only. “Sandcastles in the Storm: Revisiting
the (Im)possibility of Strong Watermarking” is ACL `2025.acl-long.1436`, DOI
`10.18653/v1/2025.acl-long.1436`, and is citation-only. “Watermark Smoothing
Attacks against Language Models” is ACL `2025.findings-emnlp.264`, DOI
`10.18653/v1/2025.findings-emnlp.264`, with author-linked Apache-2.0 repository
revision `5acda5f1f27ddebe758d051537b0a59982f89b22`; it remains citation/future
threat-model only and no attack implementation is integrated.

UWBench was identified as “Analyzing and Evaluating Unbiased Language Model
Watermark,” arXiv:2509.24048, by Yihan Wu, Xuehao Cui, Ruibo Chen, and Heng
Huang. The retrieved paper page did not provide an author repository; none is
recorded. Its distribution-preservation, repeated-query, SPMG, fixed-key,
detectability, stringent-FPR, paraphrase-variance, and token-modification axes
remain related-work/future validation rather than v1 implementation claims.
