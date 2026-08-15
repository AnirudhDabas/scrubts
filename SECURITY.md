# Security policy

## Reporting

Do not post exploit details, malicious fixtures, or sensitive user artifacts in
a public issue. This repository does not currently publish a dedicated security
email or a verified private-vulnerability-reporting endpoint. Open a GitHub
issue titled `Security contact request` containing only the affected scrub
version, a short non-sensitive impact category, and a way to contact you. The
maintainer can then arrange a private channel.

## Scope

Security-sensitive reports include parser crashes or resource exhaustion on
hostile artifacts, terminal-control injection, path or sidecar confusion,
unexpected network access, unsafe archive verification or extraction, C2PA
state upgrades that could misrepresent invalid evidence as valid, and release
workflow or provenance failures that could substitute unreviewed bytes.

The Rust CLI, report import and rendering, C2PA integration, release packager
and verifier, GitHub Actions workflows, and WaterLARP handling of untrusted
cached artifacts are in scope. Provider services, upstream dependencies, model
hosts, and third-party datasets should also be reported to their maintainers
when the defect is upstream.

scrub inspection is local and does not require telemetry or network access.
Public native release artifacts are not established until a tagged release is
actually published; the current release contract and verification boundaries
are documented in [`docs/RELEASE_INTEGRITY.md`](docs/RELEASE_INTEGRITY.md).
