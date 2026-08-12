# scrub.ts repository instructions

scrub.ts is research-grade provenance forensics and watermark-evaluation software.
Correctness and evidence semantics outrank feature count.

## Scientific rules

1. Never report ABSENT unless a supported detector/check completed successfully and its documented decision semantics support absence.
2. UNKNOWN, UNSUPPORTED, INVALID, and NOT_APPLICABLE are valid outcomes.
3. Preserve raw evidence required to audit a finding.
4. Follow `docs/SOURCE_AUTHORITY.md`. Prefer primary standards/reference implementations over summaries.
5. Do not invent undocumented vendor behavior.
6. Deterministic scanners must not depend on an LLM.
7. Published experiment tables/figures must derive from committed or archived machine-readable results.
8. Do not silently modify user content.
9. Claude embedded text watermark status remains UNKNOWN unless a supported public/authorized detector can actually evaluate the artifact.
10. A transformation outcome such as REMOVED is not an observation status.

## Engineering rules

1. Implement the smallest correct thing for the approved milestone.
2. Do not create speculative traits, factories, managers, registries, plugins, or empty modules.
3. Prefer explicit types and typed errors over stringly behavior.
4. Keep stdout machine-clean when a command promises JSON/JSONL; diagnostics go to stderr.
5. Deterministic code must stay deterministic for the same input + config + version.
6. No unsafe Rust in v0.1 without an ADR and demonstrated necessity.
7. New production dependencies require a stated reason, license check, and maintenance judgment.
8. No network/telemetry/database/daemon behavior unless an approved spec requires it.
9. Parsers/scanners must treat inputs as hostile and receive adversarial tests.

## Sources, research, and licensing

- Before adapting/reimplementing a method, check `research/sources.yaml`.
- Record exact upstream revision/license/integration mode.
- Prefer adapters or independent reimplementations validated against reference behavior over unexplained copied code.
- Preserve required LICENSE/NOTICE attribution.
- Do not commit third-party PDFs, datasets, model outputs, or fixtures unless redistribution rights are clear.

## Git policy

The human maintainer owns staging and history.
Automated coding tools may inspect read-only Git state (`status`, `diff`, `log`, `show`) but must not stage, commit, push, tag, merge, rebase, reset, switch/checkout branches, stash, or rewrite history.

## Planning

For multi-file/multi-step work, read `docs/PLANS.md` and maintain a milestone plan in `docs/plans/`.
Do not expand scope beyond that plan without explicitly recording the decision.

## Verification

Run the narrowest relevant tests first, then the repository quality gate before declaring work complete.
Report exact checks executed and unresolved limitations.

## Documentation style

Write claims that are falsifiable and sourced.
Avoid marketing adjectives, fake certainty, repetitive comments, and generic generated prose.
Explain why behavior exists when the reason is non-obvious.

## Review priorities

Prioritize:
1. scientifically misleading states/claims;
2. source-authority or conformance mistakes;
3. nondeterminism and irreproducibility;
4. security/parser issues;
5. correctness bugs;
6. over-architecture and dependency creep;
7. weak tests/docs.
