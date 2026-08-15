# Launch README

## Goal

Rewrite the repository front door so a new reader can understand scrub's use,
evidence semantics, current Claude claims audit, proof surface, and deeper
engineering without changing any detector, report, research, or release
behavior.

## Non-goals

- New scanners, provider detectors, evidence, schemas, proof claims, fixtures,
  benchmarks, WaterLARP results, release artifacts, or dependencies.
- Claims that Unicode identifies Claude, public SynthID has provider parity,
  C2PA establishes authorship, or an unavailable detector establishes absence.
- Publication, packaging, Git history, or changes to the nine pre-existing
  untracked human-owned files.

## Sources / authority

- `AGENTS.md`, `docs/SOURCE_AUTHORITY.md`, and the accepted ADRs govern claim
  semantics.
- `docs/plans/0014-mega-d-a-claude-claims-audit.md` and
  `research/claude-watermark-claims/` govern the bounded audit and fixture.
- `docs/plans/0015-mega-d-b-cli-presentation.md` and the production CLI govern
  all terminal output.
- `evidence/claims.json`, `docs/specs/product-proof.md`, `CONFORMANCE.md`, the
  C2PA and WaterLARP specifications, and release-integrity contracts govern
  depth and reproducibility claims.
- Anthropic's live primary article, retrieved on 2026-08-15, governs current
  time-sensitive provider wording.

## Current state

At `8a243b4aa618794dd2235e3dd5e12b140ba8f558`, the README leads with an
abstract project description and presents detailed scope before a concrete
example. The production CLI already has a launch-quality default projection,
the bounded Claude claims audit contains seven claim records across six
convenience-sample sources, and `just prove` executes a 16-row public claim
ledger. A historical four-platform preflight exists at an earlier revision, but
no public v0.1 release is established.

The final integration pass began on `main` at committed HEAD
`2fcf802c03f736454244b12f4caaa3d952213a4e`. By that baseline, the prelaunch
install-surface milestone was committed. The source-install command
`cargo install --path crates/scrub --locked` now installs a real `scrub`
executable with successful root help, `inspect` help, and package-derived
version output. The corrected launch README remained uncommitted and already
contained the verified hero, claim boundaries, historical CI evidence,
limitations, attribution, and quiet ending.

## Design

Keep the README Markdown-native and terminal-first. Lead with a faithful
contiguous excerpt from the controlled U+200B fixture's default output, compress
the time-sensitive why-now context, and put the source-install Quick Start
before the detailed bounded audit or architecture. Then move through the audit,
evidence model, executable claim ledger, grouped engineering depth, C2PA and
WaterLARP boundaries, limitations, attribution, and reproduction links.
Describe external workflow contracts separately from scoped historical
external evidence and current-HEAD results.

## Acceptance criteria

- The first terminal block is a contiguous subset of current output from the
  controlled fixture and visibly separates Unicode `PRESENT` from Claude
  `UNKNOWN`.
- A reader reaches a verified source install and the installed command
  `scrub inspect <artifact>` before the detailed claims audit or architecture.
- The audit is explicitly bounded, time-bounded, and a convenience sample;
  both unsupported claims and accurately limited counterexamples are present.
- Every command and link is real at the expected revision; no release binary
  or untracked documentation is advertised.
- Public-reference, provider, C2PA, WaterLARP, novelty, and authorship
  boundaries remain explicit.
- The README remains compact enough to scan while exposing proof and deeper
  engineering after the basic use case.

## Implementation steps

1. Verify repository state, governing documentation, CLI behavior, proof
   claims, and current Anthropic primary-source wording.
2. Preserve the corrected README while moving source installation and the
   installed command ahead of the detailed audit and architecture.
3. Verify a fresh source install plus root help, `inspect` help, version, and
   controlled-fixture output from the installed executable.
4. Check local and external links, run the focused claim verifier, `just prove`,
   `just check`, and Markdown/Git checks, then perform the blueprint self-review.
5. Record the integrated outcome and final scope state here.

## Validation

```text
cargo run --offline --quiet -p scrub -- inspect research/claude-watermark-claims/fixtures/controlled-u200b.txt
cargo install --path crates/scrub --locked --root <fresh-isolated-root>
<fresh-isolated-root>/bin/scrub --help
<fresh-isolated-root>/bin/scrub inspect --help
<fresh-isolated-root>/bin/scrub --version
<fresh-isolated-root>/bin/scrub inspect research/claude-watermark-claims/fixtures/controlled-u200b.txt
python tools/verify_claude_watermark_claims.py
just prove
just check
git diff --check
git diff -- README.md
git diff -- docs/plans/0016-launch-readme.md
git diff --stat
git status --porcelain=v1 -uall
git diff --cached --name-only
```

README links are checked against tracked local paths and live external targets
where network access is available.

## Risks / open questions

- Anthropic's live page can change after the dated check; wording must remain
  explicitly time-bounded.
- Historical workflow evidence can be generalized beyond its revision,
  fixtures, platforms, or bounded campaign; the README must state each scope
  and distinguish it from workflow source and current-HEAD results.
- The repository contains useful untracked launch documents, but linking them
  would violate the requirement to use only committed/existing proof surfaces.
- A source install may contact the crates.io index when Cargo's cache is
  incomplete; the installed `scrub` inspection path remains local and has no
  runtime network behavior.

## Outcome

Authored, corrected, and finally integrated `README.md` as a 1,731-word
terminal-first front door. The opening uses a faithful 24-line contiguous
subset of the production CLI's controlled-fixture output. A compressed why-now
context now leads directly to source installation and the installed command
`scrub inspect <artifact>` before the detailed bounded audit or architecture.
The remaining flow is claims audit, evidence model, executable claim ledger,
grouped engineering depth, C2PA and WaterLARP boundaries, limitations,
attribution, and focused reproduction commands.

Anthropic's official article was retrieved on 2026-08-15 and still described
future Claude text watermarking, no hidden characters, an Anthropic-keyed
SynthID-Text-family mechanism, a detection API coming “soon,” and involvement
rather than authorship semantics. The live page returned HTTP 200. The README
dates the statement and links the primary source without treating its current
HTML identity as frozen repository evidence.

Validation completed:

- controlled default excerpt: faithful contiguous subset after newline
  normalization;
- above-the-fold prose around the transcript: 120 words;
- local README links: all targets tracked, both heading anchors resolved;
- repository clone target and historical run URLs: resolved through
  authenticated GitHub access;
- fresh `cargo install --path crates/scrub --locked`: PASS; the installed
  executable reported `scrub 0.1.0`, both help surfaces exited successfully,
  and controlled-fixture inspection preserved Unicode `PRESENT` separately
  from Claude `UNKNOWN`;
- `python tools/verify_claude_watermark_claims.py`: PASS, seven claims across
  six sample sources;
- `just prove`: PASS, 16/16 and `PROOF_COMPLETE`;
- `just check`: PASS (`cargo fmt --check`, warning-denied workspace Clippy, and
  all workspace tests);
- `git diff --check`: PASS.

The bounded correction records successful historical three-OS determinism and
Linux fuzz-smoke runs at revision
`a47994e334aecb868c5f0b07a2cf97da8b09950f`, plus a successful four-platform
release preflight at revision `24af24a72adc632852e5fd2114725b28bd3002f1`.
Their authenticated GitHub run records, jobs, revisions, and canonical URLs
were verified; current-HEAD, general-equivalence, absence-of-bugs, public
release, attestation, immutable-release, and platform-signing boundaries remain
explicit. The correction also removes the star CTA, separates weak-signal
conditions from complete rewriting, sharpens the audit motivation, and removes
the internal source command from the opening excerpt. No fresh bounded README
review was started.

No code, schema, detector, research data, evidence capture, workflow, release,
dependency, staging, or Git-history behavior changed. The nine pre-existing
untracked files remain untouched.

The final integration pass used committed HEAD
`2fcf802c03f736454244b12f4caaa3d952213a4e` on `main` as its baseline and left
that HEAD unchanged. The official Anthropic article again returned HTTP 200 on
2026-08-15 and retained the time-sensitive mechanism, key, hidden-character,
forthcoming-API, weak-signal, complete-rewrite, and inference-boundary wording
used by the README. Authenticated run inspection reconfirmed all three
historical workflow URLs, successful conclusions, and exact cited revisions.
Nothing was staged or committed.
