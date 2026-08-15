# Verifying scrub release integrity

scrub release archives expose four separate verification layers. Passing one
does not imply that another passed.

`tools/release.py verify-package` verifies the exact scrub package members,
their metadata/byte relationships, and the canonical ZIP or tar+gzip
representation emitted by scrub's packager. It is a verifier for this bounded
release format, not a general-purpose safe ZIP/tar parser.

## Archive checksum

From the directory containing a downloaded archive and `SHA256SUMS`, use a
shasum-compatible checker, for example:

```bash
sha256sum --check SHA256SUMS
```

This checks exact local bytes against the checksum file. The checksum file by
itself does not authenticate who published it.

## GitHub build provenance

For a future public release, verify the downloaded archive's GitHub/Sigstore
artifact attestation:

```bash
gh attestation verify ./PATH/TO/ARCHIVE -R OWNER/REPO
```

This verifies an attestation whose subject is the archive uploaded by the tag
workflow. It does not establish Apple Developer ID, Apple notarization, Windows
Authenticode, human identity, or a byte-for-byte independently reproducible
compiler build.

## Immutable GitHub release membership

After publication, verify that GitHub recognizes the release as immutable and
that a local archive is one of its locked assets:

```bash
gh release verify vX.Y.Z -R OWNER/REPO
gh release verify-asset vX.Y.Z ./PATH/TO/ARCHIVE -R OWNER/REPO
```

These commands verify public release membership/integrity, not build
provenance. They cannot succeed for an ordinary mutable draft.

## Platform-vendor signing

The v0.1 package contract records these statuses explicitly:

- Apple Developer ID: `not_provided`
- Apple notarization: `not_provided`
- Windows Authenticode: `not_provided`

Unsigned at the platform-vendor layer is not synonymous with unsafe. GitHub
build provenance remains useful, but it does not replace platform trust UI or
vendor certificates.

## Operator prerequisite and handoff

Before publishing v0.1, repository release immutability MUST be enabled in
GitHub. The tag workflow validates and builds all four native packages, smokes
the extracted binaries, attests the exact archives, assembles the manifest and
checksums, and creates or updates one draft release. It never publishes.

The human operator must inspect the complete draft and publish it. Until that
future GitHub execution occurs, the cross-platform packages, artifact
attestations, draft release, release attestation, and immutable release are
**NOT YET ESTABLISHED**.
