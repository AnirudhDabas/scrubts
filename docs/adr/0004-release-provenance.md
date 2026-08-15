# ADR 0004: release provenance

**Status:** accepted

## Decision

Public scrub releases are built by CI from the exact commit addressed by a
maintainer-created version tag. The workflow produces deterministic native
archives, SHA-256 checksums, and GitHub build attestations for the exact tag-mode
archives; it stops at a draft for human inspection and publication.

The v0.1 contract does not provide an SBOM, Apple Developer ID or notarization,
Windows Authenticode, or an independently reproducible compiler build. A future
release may add those layers only with a defined consumer and verification
contract. Tag signatures are not claimed because the workflow does not enforce
them.

## Wording rule

Build provenance or attestation is not called a "reproducible build" until
independent byte-for-byte rebuilds actually demonstrate that property.

## Why

A project about provenance should make its own release origin unusually easy
to inspect.

The exact archive, assembly, permission, and publication boundaries are in
[`docs/specs/mega-c-release-integrity.md`](../specs/mega-c-release-integrity.md)
and [`docs/RELEASE_INTEGRITY.md`](../RELEASE_INTEGRITY.md).
