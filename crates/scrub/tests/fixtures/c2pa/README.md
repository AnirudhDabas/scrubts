# C2PA fixture provenance

All hashes below are SHA-256 over the exact committed bytes.

## Official public testfiles

The four `public-testfiles/*.jpg` assets come from the C2PA official
`public-testfiles` repository at commit
`22beccc075707475b038d8789d0136c009e43143`, under
`legacy/1.4/image/jpeg/`. They are covered by `public-testfiles/LICENSE`
(CC-BY-SA-4.0):

- `legacy/1.4/image/jpeg/adobe-20220124-CA.jpg`: Git blob
  `c0fc9ac13155427f8111e7b1d24bdbcebf59a8a3`; 178,709 bytes; SHA-256
  `cafc48c53e651f7ba4622d1f72783827074211e42b9634cc863ec3be3c7651b3`;
- `legacy/1.4/image/jpeg/adobe-20220124-E-dat-CA.jpg`: Git blob
  `6284416b5a226538817f10996245b78f6eaa2b5c`; 178,709 bytes; SHA-256
  `dae9d121060cec4b6f27ee8acda85ad461cf75f2261d90b463319b787342d7f9`;
- `legacy/1.4/image/jpeg/adobe-20220124-E-sig-CA.jpg`: Git blob
  `142f6dc7ba854af189b84ee58859de732aa3efef`; 178,709 bytes; SHA-256
  `0d4c2774f1b7e94b9613bb952b0a76b6a178d22ac6d206d257d2af1376cbbff2`;
- `legacy/1.4/image/jpeg/adobe-20220124-E-clm-CAICAI.jpg`: Git blob
  `57e88677f57f9b13bc64a4d4627b6a7902c80498`; 656,258 bytes; SHA-256
  `b3ff3f00c66602280977d3e4d962a836d33ab83953d0009f7c1e6490d0065feb`.

The first is known-good; the others are the upstream data-, signature-, and
claim-tampered vectors.

## Released SDK source and authored derivatives

The following files are exact copies from `contentauth/c2pa-rs` release commit
`ae0c3fde8ea399bf7f12379bb44e38b2738b8369` (`c2pa-v0.90.12`):

- `sdk/tests/fixtures/sample1.png`, stored as `unsigned/sample.png`: Git blob
  `cfd2f19ab800fc2516f6ccdc836e4a53a154b6ff`; 299,257 bytes; SHA-256
  `0bd72c972c14e5fd27d8473c5599e801031576e1398f384c40c3394696a2619a`;
- `sdk/tests/fixtures/sample1.svg`, stored as `unsigned/sample.svg`: Git blob
  `5c49e9e33c1c232fb6f4f29500e29b866245a3f4`; 26,580 bytes; SHA-256
  `86a722290d12f661c619063cd8cb0137a720671f778596093bb7ac91525b18fe`;
- `sdk/tests/fixtures/ocsp.jpg`, stored as `c2pa-rs/ocsp.jpg`: Git blob
  `42c9e692ae453cdf782945691b96a226a8079c4d`; 285,562 bytes; SHA-256
  `49a6b089bf3fe610960ef91b2beb81c86b62e7d531c97f33fc029841d864b2cb`;
- `sdk/tests/fixtures/ocsp_with_assertion.jpg`, stored as
  `c2pa-rs/ocsp_with_assertion.jpg`: Git blob
  `368005f2cd784e44218287c851536c79433d9fdd`; 599,791 bytes; SHA-256
  `210fb95c6a766d3cd89ef0583898ec7248fe60f0ed651af216fb270cd9cbe17a`.

The OCSP fixtures exercise a COSE-stapled response and a CertificateStatus
assertion respectively. They are test inputs, not copied implementation code.

The omitted-claim architectural regression creates a test-only derivative of
`ocsp_with_assertion.jpg` in the temporary test directory. It replaces the two
`c2pa.hash.data` labels belonging to the untimestamped CertificateStatus claim
with the equal-length `c2pa.actions_1`. This makes that claim fail
`Manifest::from_store` materialization while the active and intermediate
Manifests retain their reference to its label. The derivative is 599,791 bytes,
SHA-256 `0c9948826452dc34f43ee252e04834fc7b903c5c2b25664890829881c81e3e5f`;
its exact six source-label offsets and first-two replacement rule are asserted
by the integration test. It is not an additional source-authority fixture.

`generated/signed.png` and `generated/signed.svg` are test-only authored
derivatives of the exact `sample1.png` and `sample1.svg` assets. They were
produced once with c2pa-rs
0.90.12's in-memory `Builder::sign` and the release tree's non-production
Ed25519 test certificate/key (`sdk/tests/fixtures/certs/ed25519.pub` and
`ed25519.pem`). The frozen definition used claim version 2, SHA-256, fixed UUID
`00000000-0000-4000-8000-000000000002`, fixed instance ID
`00000000-0000-4000-8000-000000000001`, and one `c2pa.created` actions-v2
assertion with digital source type `http://c2pa.org/digitalsourcetype/empty`.
These generated derivatives do not correspond to upstream Git blobs. Their
hashes are
`276e64f0ba1f0ed3cd153f5fb166fb1864fadd03fd6d3cd5427cc77fc935fdb0`
and `296c5e254427620ff3aef3176adf64e0311986775668013c33526c8ad1fc6fde`.
They supplement, but do not replace, the independently authored official JPEG
vectors.

All files in this section are covered by the upstream
MIT-or-Apache-2.0 terms reproduced as `LICENSE-MIT` and `LICENSE-APACHE`.
The private key is intentionally not committed and is not used by production.
