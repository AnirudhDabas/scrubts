# C2PA 2.4 provenance inspection

## Scope and authority

This contract defines inspection-only C2PA findings for C2PA Technical
Specification 2.4 Appendix A.8 unstructured text and embedded C2PA in PNG,
JPEG, and SVG. It does not implement Claude's embedded text watermark and does
not infer authorship, truth, originality, or whether content is AI-generated.

Authority is, in order:

1. C2PA Technical Specification 2.4, including its normative validation status
   codes and Appendix A embedding rules;
2. released `c2pa` crate 0.90.12 at tag `c2pa-v0.90.12`, commit
   `ae0c3fde8ea399bf7f12379bb44e38b2738b8369`;
3. official C2PA/Content Authenticity Initiative fixtures at that revision;
4. `c2pa-text` 3.0.0 only as an independent A.8 byte-order and mapping
   cross-check. It is not a production dependency or authority.

The exact C2PA HTML reviewed on 2026-08-12 is archived in the ignored research
library at `research/library/c2pa/2.4/C2PA_Specification.html`: 1,065,169 bytes,
SHA-256 `d55caebd96206f0de667962a4bab7098c6b6468fba68f8b55bcdd3a12d1ed26d`.
The specification is CC BY 4.0. Appendix A.8 explicitly says the method remains
under review and may change after implementation and interoperability feedback.

Anthropic's official documentation describes embedded model-level text
watermarks and signed C2PA file provenance as two different mechanisms. Its
2026-08-14 technical article identifies the text-watermark family as a version
of the SynthID-Text approach and says it does not append hidden characters; the
exact provider detector remains unavailable. Neither statement changes Unicode
or C2PA evidence semantics. A Unicode or C2PA result is therefore never reported
as Claude embedded-text-watermark detection.

## Artifact classification

Classification is deterministic and content-first:

- the exact PNG signature selects PNG;
- `FF D8 FF` selects JPEG;
- complete valid UTF-8 whose first XML element is `svg` selects SVG;
- other complete valid UTF-8 is treated as A.8-applicable unstructured text;
- malformed non-binary bytes use a case-insensitive `.txt` suffix only as the
  narrow fallback needed to classify an A.8-applicable text artifact;
- other bytes are unsupported for this milestone.

Filename extensions do not override contradictory content. Except for the
malformed-text fallback above, they are only SDK hints after content
classification. SVG classification permits an optional UTF-8 BOM and a linear
sequence of ASCII whitespace, an XML declaration, XML processing instructions,
comments, and a bounded DOCTYPE before the root element. Processing-instruction
targets satisfy XML 1.0 Fifth Edition `Name`; the reserved target `xml` is
rejected case-insensitively outside the separately recognized XML declaration.
The first document element must still be `svg`; another XML root and text that
merely contains a later `<?...?>` or `<svg` remain text. A structured non-SVG
text format is not an A.9 support claim; scrub only evaluates whether its
complete valid Unicode text contains an A.8 wrapper.

## Finding order and version

All five mechanisms use version `2.4`. Canonical output order is:

1. `unicode.bidi_control`;
2. `unicode.default_ignorable_code_point`;
3. `unicode.normalization.nfc_difference`;
4. `unicode.normalization.nfkc_difference`;
5. `c2pa.text_manifest_wrapper`;
6. `c2pa.manifest_store`;
7. `c2pa.manifest_validation`;
8. `c2pa.hard_binding`;
9. `c2pa.credential_trust`.

The first four identifiers and their semantics are unchanged. The report layer
uses this explicit mechanism order, followed by lexical order for unknown future
mechanisms. Evidence names remain lexically ordered by the report contract.

## `c2pa.text_manifest_wrapper`

This finding evaluates only Appendix A.8 carriers.

- `present`: complete valid UTF-8 contains exactly one syntactically valid
  `C2PATextManifestWrapper` and no detected C2PATXT magic candidate is corrupt.
- `absent`: complete valid UTF-8 contains no valid wrapper and no corrupt magic
  candidate. Ordinary U+FEFF, VS15/VS16, random variation selectors, and wrong
  magic remain absent.
- `invalid`: UTF-8 is malformed/incomplete; a full C2PATXT magic or a truncated
  exact magic prefix is followed by a malformed/incomplete wrapper; or more
  than one valid wrapper exists. Corruption remains invalid when another valid
  wrapper is also present.
- `not_applicable`: the content is PNG, JPEG, or SVG, which uses a different
  supported embedding mechanism.
- `unsupported`: the artifact cannot be classified as complete Unicode text or
  one of the supported binary formats.

The wrapper is U+FEFF followed by one contiguous variation-selector block. Each
selector decodes to one byte: U+FE00..U+FE0F map to 0..15 and
U+E0100..U+E01EF map to 16..255. The first eight bytes are `C2PATXT\0`, version
is 1, and the next four bytes are an unsigned big-endian manifest byte length.
Exactly that many following selectors form the payload. Extra contiguous
selectors are invalid because 2.4 defines no padding field. This deliberately
does not adopt `c2pa-text` 3.0.0's permissive trailing-padding extension.

Appendix A.8.2.1 specifies quantity "Zero or one." The canonical failure code
for the second valid wrapper is `manifest.text.multipleWrappers`. `present`
evidence is exactly:

- `first_wrapper`: compact JSON with key order
  `original_byte_offset`, `original_byte_length`, `normalized_byte_offset`,
  `normalized_byte_length`, `wrapper_version`, `declared_manifest_length`,
  `manifest_store_sha256`;
- `wrapper_count`: total valid wrapper count.

Normalized coordinates are bytes in NFC-normalized UTF-8. The normalized start
is computed from the complete prefix ending before U+FEFF using the pinned
Unicode 17 normalization implementation. U+FEFF is a normalization boundary and
the wrapper scalars are NFC-stable, so its normalized length equals its original
UTF-8 length. `absent` has no evidence. `invalid` has `candidate_count` and a
compact `first_error` object containing `code` and `original_byte_offset`; an
UTF-8 failure instead has only the existing `utf8_validation` evidence. An
invalid asset never retains partial `first_wrapper` or `wrapper_count`
evidence. `candidate_count` is the total number of valid or corrupt C2PATXT
candidates, including all candidates after the first error.

Scanning is single-pass over scalar boundaries, skips each contiguous selector
run once, uses checked arithmetic, and never allocates from the declared length.
The physically available selector count is checked before payload hashing.

## `c2pa.manifest_store`

This asks whether an embedded Manifest Store is structurally obtainable.

- `present`: the released SDK extracted a store from supported PNG/JPEG/SVG
  bytes and made its active store structurally available with
  `verify_after_reading=false`.
- `absent`: the released SDK completed embedded-store lookup for a supported
  PNG/JPEG/SVG artifact and returned its typed `JumbfNotFound` result.
- `invalid`: a supported binary carrier/store is malformed and the SDK cannot
  structurally obtain it.
- `unsupported`: the format is outside the milestone, or A.8 carrier bytes were
  extracted but released stable APIs cannot parse them with text semantics.
- `not_applicable`: A.8 text has no valid wrapper.

Store presence comes only from a parse/presence `Reader` whose explicit Context
disables post-read validation. In c2pa-rs 0.90.12 this skips
`Store::verify_store`, including certificate-profile, OCSP, CertificateStatus,
trust, and hard-binding evaluation, while retaining parsed claims, the
provenance label, and public `Manifest` structure. Validation can therefore
neither suppress nor change public store presence. `present` evidence is
exactly `claim_version`, `manifest_count`, and
`manifest_store_sha256`. The digest is over the exact embedded JUMBF bytes
returned by the SDK. A constructor or parser success is not cryptographic
validity.

## `c2pa.manifest_validation`

This asks whether the structurally obtained active manifest/store satisfies the
SDK's structural and cryptographic checks under scrub's fixed configuration.

- `present`: a separate released-SDK validation `Reader` reports `Valid` (or
  `Trusted`, though scrub's policy cannot produce it) on a reproducible
  validation basis.
- `invalid`: that validation `Reader` reports `Invalid` on a reproducible
  validation basis.
- `unknown`: the parse/presence Reader cannot establish a reproducible
  validation basis. scrub does not run the validation Reader for this case, so
  a current certificate or revocation result cannot become a permanent
  artifact claim.
- `not_applicable`: no structurally available store exists.
- `unsupported`: stable released APIs cannot perform the applicable validation,
  including A.8 in this milestone.

For c2pa-rs 0.90.12, scrub proves coverage from the stable public representation
instead of assuming `Reader::manifests()` is complete. It starts at
`Reader::active_label`, recursively follows every public
`Manifest::ingredients()[].active_manifest()` reference, and requires every
reached label to resolve through `Reader::get_manifest`. This mirrors the
released `Store::verify_store` active-claim ingredient traversal. A failed
`Manifest::from_store` materialization therefore leaves a referenced label
unresolved and cannot authorize Phase 2; unreferenced Store claims are outside
that validation traversal. The public `manifest_count` remains the count of
successfully materialized public Manifests, not a private Store claim count.

Every reached Manifest must also have a cryptographically validated RFC 3161
signature timestamp exposed by stable `Manifest::signature_info` and must not
contain a `c2pa.certificate-status` assertion. The SDK uses the validated
timestamp for signing-certificate validity and a COSE-stapled OCSP response. In
contrast, 0.90.12 preprocesses every CertificateStatus response with
`signing_time=None`, which uses the current clock even if a claim timestamp is
later available. Untimestamped certificate and OCSP paths also use the current
clock. Unresolved coverage, legacy time bases, and otherwise unexposed time
bases are conservatively `unknown` with the existing
`validation_time_basis=not_reproducible` evidence.

On a reproducible basis, evidence is exactly `failure_codes`,
`informational_codes`, `success_codes`, `validation_state`, and
`validation_time_basis=validated_timestamp`. Each code value is compact JSON
with key order `codes`, `total`, `truncated`; codes are sorted, deduplicated,
and limited to the first 256. If validation terminates before structured
results are available, evidence is `validation_state=invalid` and
`validation_time_basis=validated_timestamp`. On a non-reproducible basis,
evidence is exactly `validation_time_basis=not_reproducible` and contains no
SDK result from the current clock. `validationTime`, explanations, certificate
payloads, and SDK JSON are never emitted.

## `c2pa.hard_binding`

This asks whether the active claim's hard binding matches the exact inspected
artifact under the applicable C2PA rules.

- `present`: the active-manifest status codes include a successful applicable
  data, general-box, or BMFF hash result and no active hard-binding failure.
- `invalid`: an applicable active binding mismatch or malformed-binding status
  is present.
- `unknown`: validation was not run because the applicable credential or
  revocation path lacks a reproducible time basis. Evidence is exactly
  `validation_time_basis=not_reproducible`.
- `not_applicable`: no usable active manifest or active hard binding exists.
- `unsupported`: the binding mechanism cannot be evaluated, including A.8 in
  released `c2pa` 0.90.12.

Evidence is exactly `algorithm`, `binding_type`, and `validation_code` when the
SDK exposes the corresponding structured result. Algorithm is `sdk_selected`
when the stable status API establishes the binding result but does not expose
the resolved assertion algorithm without widening the public manifest surface.
Signature validity never implies hard-binding success.

C2PA 2.4 A.8 hard binding requires exclusion offsets in NFC-normalized UTF-8
byte coordinates, exact wrapper-boundary selection, wrapper removal, NFC of the
remaining visible text, UTF-8 encoding, and assertion-algorithm hashing. The
released crate has no stable text handler, and its raw `.c2pa` handler would
validate a different asset with different hard-binding semantics. scrub does
not use that path and reports A.8 hard binding `unsupported`.

## `c2pa.credential_trust`

Trust is distinct from integrity.

- `unknown`: manifest integrity is valid, but scrub v0.1 has no pinned C2PA
  trust list or trust-time policy. Evidence is exactly
  `trust_policy=not_configured`.
- `not_applicable`: no valid signed manifest exists.
- `unsupported`: applicable manifest validation itself is unsupported.

This milestone never emits `present` for credential trust. It does not load OS
trust roots, ambient C2PA settings, user roots, or a live trust list. An unknown
signer is not described as malicious.

## Offline released-SDK configuration

Production uses exactly `c2pa = "=0.90.12"` with default features disabled and
only `rust_native_crypto`. It constructs a fresh `Settings::new()` and Context
for each phase. Both phases set:

- `verify.verify_trust=false`;
- `verify.verify_timestamp_trust=false`;
- `verify.ocsp_fetch=false`;
- `verify.remote_manifest_fetch=false`;
- `core.allowed_network_hosts=[]`;
- empty C2PA and CAWG trust/user/allowed/config values;
- `core.decode_identity_assertions=false`.

The parse/presence phase sets `verify.verify_after_reading=false`. A separate
validation phase sets it to `true` only after the first phase establishes the
reproducible basis above.

The build excludes `file_io`, all default HTTP features,
`fetch_remote_manifests`, PDF, `unstable_plain_text`, and
`unstable_structured_text`. Production calls only
`Reader::from_context(...).with_stream(...)` and SDK in-memory JUMBF extraction
over cursors borrowing the same retained artifact bytes. It never calls file
APIs, opens a sidecar, resolves a URL, invokes a tool, or reads neighboring
files. Explicit settings replace, rather than overlay, ambient thread-local
settings.

## Same-byte and failure invariants

One 65,536-byte read loop remains authoritative for artifact SHA-256, byte
length, Unicode decoding, and C2PA. PNG/JPEG signatures activate fallible exact
byte retention while their prefix is still in the fixed sniff buffer. SVG and
A.8 reuse the exact valid-UTF-8 buffer. SDK cursors borrow those same retained
bytes; the path is never reopened. Unsupported malformed bytes are not
whole-buffered merely for C2PA.

Read, checked-arithmetic, or scrub-owned allocation failure fails the command.
It never becomes `absent`, `unknown`, or `invalid`. Binary C2PA evaluation is
independent of UTF-8 validity, so PNG/JPEG can have four invalid Unicode
findings while their C2PA layers remain valid.

## Interpretation limits

A valid hard binding establishes only that the applicable signed claim is bound
to the inspected content under C2PA rules. It does not establish that Claude or
AI authored the underlying ideas, that a person did not create them, that the
file was never edited, or that the content is truthful. Presence alone does not
establish integrity, binding, or trust. Absence of C2PA does not establish human
authorship.
