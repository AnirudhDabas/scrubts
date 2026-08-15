# Third-party notices

This notice covers third-party software in scrub.ts native release archives and
fixture-adjacent material committed for conformance or replay. It records source
metadata and attribution; it is not a legal-compliance certification.

## Native Rust release dependency inventory

The inventory below is the union of normal and build dependencies resolved for
the locked `scrub` package on the four release targets. It was derived with
Cargo 1.97.1 from `Cargo.lock` using `cargo tree -p scrub --locked --offline
--edges normal,build` for each target. Development-only dependencies and the
local `scrub-report` package are excluded. Build dependencies and procedural
macros are included.

The release targets are `x86_64-unknown-linux-gnu`,
`aarch64-apple-darwin`, `x86_64-apple-darwin`, and
`x86_64-pc-windows-msvc`. License expressions below are the packages' raw
declared Cargo metadata; this notice does not normalize legacy slash expressions
or silently choose a license where the declaration is compound.

Direct third-party dependencies are `c2pa 0.90.12`, `sha2 0.11.0`, and
`unicode-normalization 0.1.25`. The other 248 packages are resolved
transitive dependencies.

`THIRD_PARTY_LICENSES.txt` is the corresponding deterministic license-text
bundle. It maps every inventory row to the exact candidate files present in the
checksum-identified crate package or, when the package omits those files, to an
exact Cargo VCS revision and file digest. Byte-identical texts are stored once,
but every package retains its upstream filename and text digest mapping. The
independently committed reviewed mapping contract is
`third_party/license-manifest.json`; release verification requires the bundle's
exact package/file/digest membership to match it. The generator and verifier
are `tools/third_party_licenses.py`.

| Package | Version | Relationship | Declared license expression | Release target membership |
|---|---:|---|---|---|
| [abnf](https://crates.io/crates/abnf/0.13.0) | 0.13.0 | transitive | `MIT OR Apache-2.0` | all four |
| [abnf-core](https://crates.io/crates/abnf-core/0.5.0) | 0.5.0 | transitive | `MIT OR Apache-2.0` | all four |
| [adler2](https://crates.io/crates/adler2/2.0.1) | 2.0.1 | transitive | `0BSD OR MIT OR Apache-2.0` | all four |
| [aho-corasick](https://crates.io/crates/aho-corasick/1.1.5) | 1.1.5 | transitive | `Unlicense OR MIT` | all four |
| [alloc-no-stdlib](https://crates.io/crates/alloc-no-stdlib/2.0.4) | 2.0.4 | transitive | `BSD-3-Clause` | all four |
| [alloc-stdlib](https://crates.io/crates/alloc-stdlib/0.2.4) | 0.2.4 | transitive | `BSD-3-Clause` | all four |
| [asn1-rs](https://crates.io/crates/asn1-rs/0.7.2) | 0.7.2 | transitive | `MIT OR Apache-2.0` | all four |
| [asn1-rs-derive](https://crates.io/crates/asn1-rs-derive/0.6.0) | 0.6.0 | transitive | `MIT OR Apache-2.0` | all four |
| [asn1-rs-impl](https://crates.io/crates/asn1-rs-impl/0.2.0) | 0.2.0 | transitive | `MIT/Apache-2.0` | all four |
| [async-generic](https://crates.io/crates/async-generic/1.1.2) | 1.1.2 | transitive | `MIT OR Apache-2.0` | all four |
| [async-trait](https://crates.io/crates/async-trait/0.1.92) | 0.1.92 | transitive | `MIT OR Apache-2.0` | all four |
| [atree](https://crates.io/crates/atree/0.5.4) | 0.5.4 | transitive | `MIT` | all four |
| [autocfg](https://crates.io/crates/autocfg/1.5.1) | 1.5.1 | transitive | `Apache-2.0 OR MIT` | all four |
| [base16ct](https://crates.io/crates/base16ct/0.2.0) | 0.2.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [base64](https://crates.io/crates/base64/0.22.1) | 0.22.1 | transitive | `MIT OR Apache-2.0` | all four |
| [base64ct](https://crates.io/crates/base64ct/1.8.3) | 1.8.3 | transitive | `Apache-2.0 OR MIT` | all four |
| [bcder](https://crates.io/crates/bcder/0.7.7) | 0.7.7 | transitive | `BSD-3-Clause` | all four |
| [bitflags](https://crates.io/crates/bitflags/2.13.1) | 2.13.1 | transitive | `MIT OR Apache-2.0` | all four |
| [bitvec](https://crates.io/crates/bitvec/1.1.1) | 1.1.1 | transitive | `MIT` | all four |
| [bitvec-nom2](https://crates.io/crates/bitvec-nom2/0.2.1) | 0.2.1 | transitive | `MIT` | all four |
| [block-buffer](https://crates.io/crates/block-buffer/0.10.4) | 0.10.4 | transitive | `MIT OR Apache-2.0` | all four |
| [block-buffer](https://crates.io/crates/block-buffer/0.12.1) | 0.12.1 | transitive | `MIT OR Apache-2.0` | all four |
| [brotli](https://crates.io/crates/brotli/7.0.0) | 7.0.0 | transitive | `BSD-3-Clause AND MIT` | all four |
| [brotli-decompressor](https://crates.io/crates/brotli-decompressor/4.0.3) | 4.0.3 | transitive | `BSD-3-Clause/MIT` | all four |
| [btree-range-map](https://crates.io/crates/btree-range-map/0.7.2) | 0.7.2 | transitive | `MIT/Apache-2.0` | all four |
| [btree-slab](https://crates.io/crates/btree-slab/0.6.1) | 0.6.1 | transitive | `MIT/Apache-2.0` | all four |
| [byteorder](https://crates.io/crates/byteorder/1.5.0) | 1.5.0 | transitive | `Unlicense OR MIT` | all four |
| [byteordered](https://crates.io/crates/byteordered/0.6.0) | 0.6.0 | transitive | `MIT OR Apache-2.0` | all four |
| [bytes](https://crates.io/crates/bytes/1.12.1) | 1.12.1 | transitive | `MIT` | all four |
| [c2pa](https://crates.io/crates/c2pa/0.90.12) | 0.90.12 | direct | `MIT OR Apache-2.0` | all four |
| [c2pa_cbor](https://crates.io/crates/c2pa_cbor/0.77.2) | 0.77.2 | transitive | `MIT OR Apache-2.0` | all four |
| [cc-traits](https://crates.io/crates/cc-traits/2.0.0) | 2.0.0 | transitive | `MIT/Apache-2.0` | all four |
| [cfg-if](https://crates.io/crates/cfg-if/1.0.4) | 1.0.4 | transitive | `MIT OR Apache-2.0` | all four |
| [chrono](https://crates.io/crates/chrono/0.4.45) | 0.4.45 | transitive | `MIT OR Apache-2.0` | all four |
| [ciborium](https://crates.io/crates/ciborium/0.2.2) | 0.2.2 | transitive | `Apache-2.0` | all four |
| [ciborium-io](https://crates.io/crates/ciborium-io/0.2.2) | 0.2.2 | transitive | `Apache-2.0` | all four |
| [ciborium-ll](https://crates.io/crates/ciborium-ll/0.2.2) | 0.2.2 | transitive | `Apache-2.0` | all four |
| [const-hex](https://crates.io/crates/const-hex/1.19.1) | 1.19.1 | transitive | `MIT OR Apache-2.0` | all four |
| [const-oid](https://crates.io/crates/const-oid/0.10.2) | 0.10.2 | transitive | `Apache-2.0 OR MIT` | all four |
| [const-oid](https://crates.io/crates/const-oid/0.9.6) | 0.9.6 | transitive | `Apache-2.0 OR MIT` | all four |
| [coset](https://crates.io/crates/coset/0.4.2) | 0.4.2 | transitive | `Apache-2.0` | all four |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.2.17) | 0.2.17 | transitive | `MIT OR Apache-2.0` | all four |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.3.0) | 0.3.0 | transitive | `MIT OR Apache-2.0` | all four |
| [crc32fast](https://crates.io/crates/crc32fast/1.5.0) | 1.5.0 | transitive | `MIT OR Apache-2.0` | all four |
| [crypto-bigint](https://crates.io/crates/crypto-bigint/0.5.5) | 0.5.5 | transitive | `Apache-2.0 OR MIT` | all four |
| [crypto-common](https://crates.io/crates/crypto-common/0.1.6) | 0.1.6 | transitive | `MIT OR Apache-2.0` | all four |
| [crypto-common](https://crates.io/crates/crypto-common/0.2.2) | 0.2.2 | transitive | `MIT OR Apache-2.0` | all four |
| [curve25519-dalek](https://crates.io/crates/curve25519-dalek/4.1.3) | 4.1.3 | transitive | `BSD-3-Clause` | all four |
| [curve25519-dalek-derive](https://crates.io/crates/curve25519-dalek-derive/0.1.1) | 0.1.1 | transitive | `MIT/Apache-2.0` | linux-x86_64, macos-x86_64, windows-x86_64 |
| [darling](https://crates.io/crates/darling/0.23.0) | 0.23.0 | transitive | `MIT` | all four |
| [darling_core](https://crates.io/crates/darling_core/0.23.0) | 0.23.0 | transitive | `MIT` | all four |
| [darling_macro](https://crates.io/crates/darling_macro/0.23.0) | 0.23.0 | transitive | `MIT` | all four |
| [data-encoding](https://crates.io/crates/data-encoding/2.11.1) | 2.11.1 | transitive | `MIT` | all four |
| [delegate](https://crates.io/crates/delegate/0.8.0) | 0.8.0 | transitive | `MIT OR Apache-2.0` | all four |
| [der](https://crates.io/crates/der/0.7.10) | 0.7.10 | transitive | `Apache-2.0 OR MIT` | all four |
| [deranged](https://crates.io/crates/deranged/0.5.8) | 0.5.8 | transitive | `MIT OR Apache-2.0` | all four |
| [der-parser](https://crates.io/crates/der-parser/10.0.0) | 10.0.0 | transitive | `MIT OR Apache-2.0` | all four |
| [digest](https://crates.io/crates/digest/0.10.7) | 0.10.7 | transitive | `MIT OR Apache-2.0` | all four |
| [digest](https://crates.io/crates/digest/0.11.3) | 0.11.3 | transitive | `MIT OR Apache-2.0` | all four |
| [displaydoc](https://crates.io/crates/displaydoc/0.2.7) | 0.2.7 | transitive | `MIT OR Apache-2.0` | all four |
| [ecdsa](https://crates.io/crates/ecdsa/0.16.9) | 0.16.9 | transitive | `Apache-2.0 OR MIT` | all four |
| [ed25519](https://crates.io/crates/ed25519/2.2.3) | 2.2.3 | transitive | `Apache-2.0 OR MIT` | all four |
| [ed25519-dalek](https://crates.io/crates/ed25519-dalek/2.2.0) | 2.2.0 | transitive | `BSD-3-Clause` | all four |
| [either](https://crates.io/crates/either/1.17.0) | 1.17.0 | transitive | `MIT OR Apache-2.0` | all four |
| [elliptic-curve](https://crates.io/crates/elliptic-curve/0.13.8) | 0.13.8 | transitive | `Apache-2.0 OR MIT` | all four |
| [equivalent](https://crates.io/crates/equivalent/1.0.2) | 1.0.2 | transitive | `Apache-2.0 OR MIT` | all four |
| [errno](https://crates.io/crates/errno/0.3.14) | 0.3.14 | transitive | `MIT OR Apache-2.0` | macos-aarch64, macos-x86_64 |
| [extfmt](https://crates.io/crates/extfmt/0.2.0) | 0.2.0 | transitive | `Apache-2.0` | all four |
| [fastrand](https://crates.io/crates/fastrand/2.5.0) | 2.5.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [ff](https://crates.io/crates/ff/0.13.1) | 0.13.1 | transitive | `MIT/Apache-2.0` | all four |
| [flate2](https://crates.io/crates/flate2/1.1.9) | 1.1.9 | transitive | `MIT OR Apache-2.0` | all four |
| [form_urlencoded](https://crates.io/crates/form_urlencoded/1.2.2) | 1.2.2 | transitive | `MIT OR Apache-2.0` | all four |
| [funty](https://crates.io/crates/funty/2.0.0) | 2.0.0 | transitive | `MIT` | all four |
| [generic-array](https://crates.io/crates/generic-array/0.14.9) | 0.14.9 | transitive | `MIT` | all four |
| [getrandom](https://crates.io/crates/getrandom/0.2.17) | 0.2.17 | transitive | `MIT OR Apache-2.0` | all four |
| [getrandom](https://crates.io/crates/getrandom/0.3.4) | 0.3.4 | transitive | `MIT OR Apache-2.0` | all four |
| [getrandom](https://crates.io/crates/getrandom/0.4.3) | 0.4.3 | transitive | `MIT OR Apache-2.0` | all four |
| [glob](https://crates.io/crates/glob/0.3.4) | 0.3.4 | transitive | `MIT OR Apache-2.0` | all four |
| [group](https://crates.io/crates/group/0.13.0) | 0.13.0 | transitive | `MIT/Apache-2.0` | all four |
| [half](https://crates.io/crates/half/2.7.1) | 2.7.1 | transitive | `MIT OR Apache-2.0` | all four |
| [hashbrown](https://crates.io/crates/hashbrown/0.17.1) | 0.17.1 | transitive | `MIT OR Apache-2.0` | all four |
| [heck](https://crates.io/crates/heck/0.5.0) | 0.5.0 | transitive | `MIT OR Apache-2.0` | all four |
| [hex](https://crates.io/crates/hex/0.4.3) | 0.4.3 | transitive | `MIT OR Apache-2.0` | all four |
| [hex_fmt](https://crates.io/crates/hex_fmt/0.3.0) | 0.3.0 | transitive | `MIT/Apache-2.0` | all four |
| [hkdf](https://crates.io/crates/hkdf/0.12.4) | 0.12.4 | transitive | `MIT OR Apache-2.0` | all four |
| [hmac](https://crates.io/crates/hmac/0.12.1) | 0.12.1 | transitive | `MIT OR Apache-2.0` | all four |
| [http](https://crates.io/crates/http/1.5.0) | 1.5.0 | transitive | `MIT OR Apache-2.0` | all four |
| [hybrid-array](https://crates.io/crates/hybrid-array/0.4.14) | 0.4.14 | transitive | `MIT OR Apache-2.0` | all four |
| [icu_collections](https://crates.io/crates/icu_collections/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_locale_core](https://crates.io/crates/icu_locale_core/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_normalizer](https://crates.io/crates/icu_normalizer/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_normalizer_data](https://crates.io/crates/icu_normalizer_data/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_properties](https://crates.io/crates/icu_properties/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_properties_data](https://crates.io/crates/icu_properties_data/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [icu_provider](https://crates.io/crates/icu_provider/2.2.0) | 2.2.0 | transitive | `Unicode-3.0` | all four |
| [id3](https://crates.io/crates/id3/1.17.1) | 1.17.1 | transitive | `MIT` | all four |
| [ident_case](https://crates.io/crates/ident_case/1.0.1) | 1.0.1 | transitive | `MIT/Apache-2.0` | all four |
| [idna](https://crates.io/crates/idna/1.1.0) | 1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [idna_adapter](https://crates.io/crates/idna_adapter/1.2.2) | 1.2.2 | transitive | `Apache-2.0 OR MIT` | all four |
| [img-parts](https://crates.io/crates/img-parts/0.4.0) | 0.4.0 | transitive | `MIT OR Apache-2.0` | all four |
| [indexmap](https://crates.io/crates/indexmap/2.14.0) | 2.14.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [indoc](https://crates.io/crates/indoc/2.0.7) | 2.0.7 | transitive | `MIT OR Apache-2.0` | all four |
| [iref](https://crates.io/crates/iref/3.2.2) | 3.2.2 | transitive | `MIT/Apache-2.0` | all four |
| [iref-core](https://crates.io/crates/iref-core/3.2.2) | 3.2.2 | transitive | `MIT/Apache-2.0` | all four |
| [itertools](https://crates.io/crates/itertools/0.13.0) | 0.13.0 | transitive | `MIT OR Apache-2.0` | all four |
| [itoa](https://crates.io/crates/itoa/1.0.18) | 1.0.18 | transitive | `MIT OR Apache-2.0` | all four |
| [jfifdump](https://crates.io/crates/jfifdump/0.6.0) | 0.6.0 | transitive | `MIT OR Apache-2.0` | all four |
| [lazy_static](https://crates.io/crates/lazy_static/1.5.0) | 1.5.0 | transitive | `MIT OR Apache-2.0` | all four |
| [libc](https://crates.io/crates/libc/0.2.189) | 0.2.189 | transitive | `MIT OR Apache-2.0` | linux-x86_64, macos-aarch64, macos-x86_64 |
| [libm](https://crates.io/crates/libm/0.2.16) | 0.2.16 | transitive | `MIT` | all four |
| [linux-raw-sys](https://crates.io/crates/linux-raw-sys/0.12.1) | 0.12.1 | transitive | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | linux-x86_64 |
| [litemap](https://crates.io/crates/litemap/0.8.2) | 0.8.2 | transitive | `Unicode-3.0` | all four |
| [log](https://crates.io/crates/log/0.4.33) | 0.4.33 | transitive | `MIT OR Apache-2.0` | all four |
| [memchr](https://crates.io/crates/memchr/2.8.3) | 2.8.3 | transitive | `Unlicense OR MIT` | all four |
| [minimal-lexical](https://crates.io/crates/minimal-lexical/0.2.1) | 0.2.1 | transitive | `MIT/Apache-2.0` | all four |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.8.9) | 0.8.9 | transitive | `MIT OR Zlib OR Apache-2.0` | all four |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.9.1) | 0.9.1 | transitive | `MIT OR Zlib OR Apache-2.0` | all four |
| [nom](https://crates.io/crates/nom/7.1.3) | 7.1.3 | transitive | `MIT` | all four |
| [nonempty-collections](https://crates.io/crates/nonempty-collections/1.4.0) | 1.4.0 | transitive | `MIT` | all four |
| [non-empty-string](https://crates.io/crates/non-empty-string/0.2.6) | 0.2.6 | transitive | `MIT OR Apache-2.0` | all four |
| [num-bigint](https://crates.io/crates/num-bigint/0.4.8) | 0.4.8 | transitive | `MIT OR Apache-2.0` | all four |
| [num-bigint-dig](https://crates.io/crates/num-bigint-dig/0.8.6) | 0.8.6 | transitive | `MIT/Apache-2.0` | all four |
| [num-conv](https://crates.io/crates/num-conv/0.2.2) | 0.2.2 | transitive | `MIT OR Apache-2.0` | all four |
| [num-integer](https://crates.io/crates/num-integer/0.1.47) | 0.1.47 | transitive | `MIT OR Apache-2.0` | all four |
| [num-iter](https://crates.io/crates/num-iter/0.1.46) | 0.1.46 | transitive | `MIT OR Apache-2.0` | all four |
| [num-traits](https://crates.io/crates/num-traits/0.2.19) | 0.2.19 | transitive | `MIT OR Apache-2.0` | all four |
| [oid-registry](https://crates.io/crates/oid-registry/0.8.1) | 0.8.1 | transitive | `MIT OR Apache-2.0` | all four |
| [once_cell](https://crates.io/crates/once_cell/1.21.4) | 1.21.4 | transitive | `MIT OR Apache-2.0` | all four |
| [p256](https://crates.io/crates/p256/0.13.2) | 0.13.2 | transitive | `Apache-2.0 OR MIT` | all four |
| [p384](https://crates.io/crates/p384/0.13.1) | 0.13.1 | transitive | `Apache-2.0 OR MIT` | all four |
| [p521](https://crates.io/crates/p521/0.13.3) | 0.13.3 | transitive | `Apache-2.0 OR MIT` | all four |
| [parsenic](https://crates.io/crates/parsenic/0.2.1) | 0.2.1 | transitive | `Apache-2.0 OR BSL-1.0 OR MIT` | all four |
| [pct-str](https://crates.io/crates/pct-str/2.0.0) | 2.0.0 | transitive | `MIT OR Apache-2.0` | all four |
| [pem](https://crates.io/crates/pem/3.0.6) | 3.0.6 | transitive | `MIT` | all four |
| [pem-rfc7468](https://crates.io/crates/pem-rfc7468/0.7.0) | 0.7.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [percent-encoding](https://crates.io/crates/percent-encoding/2.3.2) | 2.3.2 | transitive | `MIT OR Apache-2.0` | all four |
| [pix](https://crates.io/crates/pix/0.14.0) | 0.14.0 | transitive | `MIT OR Apache-2.0` | all four |
| [pkcs1](https://crates.io/crates/pkcs1/0.7.5) | 0.7.5 | transitive | `Apache-2.0 OR MIT` | all four |
| [pkcs8](https://crates.io/crates/pkcs8/0.10.2) | 0.10.2 | transitive | `Apache-2.0 OR MIT` | all four |
| [png_pong](https://crates.io/crates/png_pong/0.10.0) | 0.10.0 | transitive | `Apache-2.0 OR Zlib` | all four |
| [potential_utf](https://crates.io/crates/potential_utf/0.1.5) | 0.1.5 | transitive | `Unicode-3.0` | all four |
| [powerfmt](https://crates.io/crates/powerfmt/0.2.0) | 0.2.0 | transitive | `MIT OR Apache-2.0` | all four |
| [ppv-lite86](https://crates.io/crates/ppv-lite86/0.2.21) | 0.2.21 | transitive | `MIT OR Apache-2.0` | all four |
| [primeorder](https://crates.io/crates/primeorder/0.13.6) | 0.13.6 | transitive | `Apache-2.0 OR MIT` | all four |
| [proc-macro2](https://crates.io/crates/proc-macro2/1.0.107) | 1.0.107 | transitive | `MIT OR Apache-2.0` | all four |
| [proc-macro-error](https://crates.io/crates/proc-macro-error/1.0.4) | 1.0.4 | transitive | `MIT OR Apache-2.0` | all four |
| [proc-macro-error-attr](https://crates.io/crates/proc-macro-error-attr/1.0.4) | 1.0.4 | transitive | `MIT OR Apache-2.0` | all four |
| [quick-xml](https://crates.io/crates/quick-xml/0.41.0) | 0.41.0 | transitive | `MIT` | all four |
| [quote](https://crates.io/crates/quote/1.0.47) | 1.0.47 | transitive | `MIT OR Apache-2.0` | all four |
| [radium](https://crates.io/crates/radium/0.7.0) | 0.7.0 | transitive | `MIT` | all four |
| [rand](https://crates.io/crates/rand/0.8.7) | 0.8.7 | transitive | `MIT OR Apache-2.0` | all four |
| [rand_chacha](https://crates.io/crates/rand_chacha/0.3.1) | 0.3.1 | transitive | `MIT OR Apache-2.0` | all four |
| [rand_chacha](https://crates.io/crates/rand_chacha/0.9.0) | 0.9.0 | transitive | `MIT OR Apache-2.0` | all four |
| [rand_core](https://crates.io/crates/rand_core/0.6.4) | 0.6.4 | transitive | `MIT OR Apache-2.0` | all four |
| [rand_core](https://crates.io/crates/rand_core/0.9.5) | 0.9.5 | transitive | `MIT OR Apache-2.0` | all four |
| [range-set](https://crates.io/crates/range-set/0.1.1) | 0.1.1 | transitive | `Apache-2.0` | all four |
| [range-traits](https://crates.io/crates/range-traits/0.3.2) | 0.3.2 | transitive | `MIT/Apache-2.0` | all four |
| [rasn](https://crates.io/crates/rasn/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [rasn-cms](https://crates.io/crates/rasn-cms/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [rasn-derive](https://crates.io/crates/rasn-derive/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [rasn-derive-impl](https://crates.io/crates/rasn-derive-impl/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [rasn-ocsp](https://crates.io/crates/rasn-ocsp/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [rasn-pkix](https://crates.io/crates/rasn-pkix/0.28.14) | 0.28.14 | transitive | `MIT OR Apache-2.0` | all four |
| [regex](https://crates.io/crates/regex/1.13.1) | 1.13.1 | transitive | `MIT OR Apache-2.0` | all four |
| [regex-automata](https://crates.io/crates/regex-automata/0.4.18) | 0.4.18 | transitive | `MIT OR Apache-2.0` | all four |
| [regex-syntax](https://crates.io/crates/regex-syntax/0.8.11) | 0.8.11 | transitive | `MIT OR Apache-2.0` | all four |
| [rfc6979](https://crates.io/crates/rfc6979/0.4.0) | 0.4.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [riff](https://crates.io/crates/riff/2.0.0) | 2.0.0 | transitive | `MIT` | all four |
| [rsa](https://crates.io/crates/rsa/0.9.10) | 0.9.10 | transitive | `MIT OR Apache-2.0` | all four |
| [rustc_version](https://crates.io/crates/rustc_version/0.4.1) | 0.4.1 | transitive | `MIT OR Apache-2.0` | all four |
| [rusticata-macros](https://crates.io/crates/rusticata-macros/4.1.0) | 4.1.0 | transitive | `MIT/Apache-2.0` | all four |
| [rustix](https://crates.io/crates/rustix/1.1.4) | 1.1.4 | transitive | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | linux-x86_64, macos-aarch64, macos-x86_64 |
| [rustversion](https://crates.io/crates/rustversion/1.0.23) | 1.0.23 | transitive | `MIT OR Apache-2.0` | all four |
| [sec1](https://crates.io/crates/sec1/0.7.3) | 0.7.3 | transitive | `Apache-2.0 OR MIT` | all four |
| [semver](https://crates.io/crates/semver/1.0.28) | 1.0.28 | transitive | `MIT OR Apache-2.0` | all four |
| [serde](https://crates.io/crates/serde/1.0.229) | 1.0.229 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_bytes](https://crates.io/crates/serde_bytes/0.11.19) | 0.11.19 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_core](https://crates.io/crates/serde_core/1.0.229) | 1.0.229 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_derive](https://crates.io/crates/serde_derive/1.0.229) | 1.0.229 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_json](https://crates.io/crates/serde_json/1.0.151) | 1.0.151 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_spanned](https://crates.io/crates/serde_spanned/1.1.1) | 1.1.1 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_with](https://crates.io/crates/serde_with/3.22.0) | 3.22.0 | transitive | `MIT OR Apache-2.0` | all four |
| [serde_with_macros](https://crates.io/crates/serde_with_macros/3.22.0) | 3.22.0 | transitive | `MIT OR Apache-2.0` | all four |
| [serde-transcode](https://crates.io/crates/serde-transcode/1.1.1) | 1.1.1 | transitive | `MIT/Apache-2.0` | all four |
| [sha1](https://crates.io/crates/sha1/0.11.0) | 0.11.0 | transitive | `MIT OR Apache-2.0` | all four |
| [sha2](https://crates.io/crates/sha2/0.10.9) | 0.10.9 | transitive | `MIT OR Apache-2.0` | all four |
| [sha2](https://crates.io/crates/sha2/0.11.0) | 0.11.0 | direct | `MIT OR Apache-2.0` | all four |
| [signature](https://crates.io/crates/signature/2.2.0) | 2.2.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [simd-adler32](https://crates.io/crates/simd-adler32/0.3.10) | 0.3.10 | transitive | `MIT` | all four |
| [slab](https://crates.io/crates/slab/0.4.12) | 0.4.12 | transitive | `MIT` | all four |
| [smallvec](https://crates.io/crates/smallvec/1.15.2) | 1.15.2 | transitive | `MIT OR Apache-2.0` | all four |
| [snafu](https://crates.io/crates/snafu/0.8.9) | 0.8.9 | transitive | `MIT OR Apache-2.0` | all four |
| [snafu-derive](https://crates.io/crates/snafu-derive/0.8.9) | 0.8.9 | transitive | `MIT OR Apache-2.0` | all four |
| [spin](https://crates.io/crates/spin/0.9.9) | 0.9.9 | transitive | `MIT` | all four |
| [spki](https://crates.io/crates/spki/0.7.3) | 0.7.3 | transitive | `Apache-2.0 OR MIT` | all four |
| [stable_deref_trait](https://crates.io/crates/stable_deref_trait/1.2.1) | 1.2.1 | transitive | `MIT OR Apache-2.0` | all four |
| [static-iref](https://crates.io/crates/static-iref/3.0.0) | 3.0.0 | transitive | `MIT/Apache-2.0` | all four |
| [static-regular-grammar](https://crates.io/crates/static-regular-grammar/2.0.2) | 2.0.2 | transitive | `MIT/Apache-2.0` | all four |
| [strsim](https://crates.io/crates/strsim/0.11.1) | 0.11.1 | transitive | `MIT` | all four |
| [subtle](https://crates.io/crates/subtle/2.6.1) | 2.6.1 | transitive | `BSD-3-Clause` | all four |
| [syn](https://crates.io/crates/syn/1.0.109) | 1.0.109 | transitive | `MIT OR Apache-2.0` | all four |
| [syn](https://crates.io/crates/syn/2.0.119) | 2.0.119 | transitive | `MIT OR Apache-2.0` | all four |
| [syn](https://crates.io/crates/syn/3.0.3) | 3.0.3 | transitive | `MIT OR Apache-2.0` | all four |
| [synstructure](https://crates.io/crates/synstructure/0.13.2) | 0.13.2 | transitive | `MIT` | all four |
| [tap](https://crates.io/crates/tap/1.0.1) | 1.0.1 | transitive | `MIT` | all four |
| [tempfile](https://crates.io/crates/tempfile/3.27.0) | 3.27.0 | transitive | `MIT OR Apache-2.0` | all four |
| [thiserror](https://crates.io/crates/thiserror/1.0.69) | 1.0.69 | transitive | `MIT OR Apache-2.0` | all four |
| [thiserror](https://crates.io/crates/thiserror/2.0.20) | 2.0.20 | transitive | `MIT OR Apache-2.0` | all four |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/1.0.69) | 1.0.69 | transitive | `MIT OR Apache-2.0` | all four |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/2.0.20) | 2.0.20 | transitive | `MIT OR Apache-2.0` | all four |
| [time](https://crates.io/crates/time/0.3.55) | 0.3.55 | transitive | `MIT OR Apache-2.0` | all four |
| [time-core](https://crates.io/crates/time-core/0.1.9) | 0.1.9 | transitive | `MIT OR Apache-2.0` | all four |
| [time-macros](https://crates.io/crates/time-macros/0.2.32) | 0.2.32 | transitive | `MIT OR Apache-2.0` | all four |
| [tinystr](https://crates.io/crates/tinystr/0.8.3) | 0.8.3 | transitive | `Unicode-3.0` | all four |
| [tinyvec](https://crates.io/crates/tinyvec/1.10.0) | 1.10.0 | transitive | `Zlib OR Apache-2.0 OR MIT` | all four |
| [tinyvec_macros](https://crates.io/crates/tinyvec_macros/0.1.1) | 0.1.1 | transitive | `MIT OR Apache-2.0 OR Zlib` | all four |
| [toml](https://crates.io/crates/toml/1.1.4+spec-1.1.0) | 1.1.4+spec-1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [toml_datetime](https://crates.io/crates/toml_datetime/1.1.1+spec-1.1.0) | 1.1.1+spec-1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [toml_parser](https://crates.io/crates/toml_parser/1.1.3+spec-1.1.0) | 1.1.3+spec-1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [toml_writer](https://crates.io/crates/toml_writer/1.1.2+spec-1.1.0) | 1.1.2+spec-1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [traitful](https://crates.io/crates/traitful/0.3.0) | 0.3.0 | transitive | `Apache-2.0 OR BSL-1.0 OR MIT` | all four |
| [typed-path](https://crates.io/crates/typed-path/0.12.3) | 0.12.3 | transitive | `MIT OR Apache-2.0` | all four |
| [typenum](https://crates.io/crates/typenum/1.20.1) | 1.20.1 | transitive | `MIT OR Apache-2.0` | all four |
| [unicode-ident](https://crates.io/crates/unicode-ident/1.0.24) | 1.0.24 | transitive | `(MIT OR Apache-2.0) AND Unicode-3.0` | all four |
| [unicode-normalization](https://crates.io/crates/unicode-normalization/0.1.25) | 0.1.25 | direct | `MIT OR Apache-2.0` | all four |
| [url](https://crates.io/crates/url/2.5.8) | 2.5.8 | transitive | `MIT OR Apache-2.0` | all four |
| [utf8_iter](https://crates.io/crates/utf8_iter/1.0.4) | 1.0.4 | transitive | `Apache-2.0 OR MIT` | all four |
| [utf8-decode](https://crates.io/crates/utf8-decode/1.0.1) | 1.0.1 | transitive | `MIT/Apache-2.0` | all four |
| [uuid](https://crates.io/crates/uuid/1.24.0) | 1.24.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [version_check](https://crates.io/crates/version_check/0.9.5) | 0.9.5 | transitive | `MIT/Apache-2.0` | all four |
| [web-time](https://crates.io/crates/web-time/1.1.0) | 1.1.0 | transitive | `MIT OR Apache-2.0` | all four |
| [windows-link](https://crates.io/crates/windows-link/0.2.1) | 0.2.1 | transitive | `MIT OR Apache-2.0` | windows-x86_64 |
| [windows-sys](https://crates.io/crates/windows-sys/0.61.2) | 0.61.2 | transitive | `MIT OR Apache-2.0` | windows-x86_64 |
| [winnow](https://crates.io/crates/winnow/1.0.4) | 1.0.4 | transitive | `MIT` | all four |
| [writeable](https://crates.io/crates/writeable/0.6.3) | 0.6.3 | transitive | `Unicode-3.0` | all four |
| [wyz](https://crates.io/crates/wyz/0.5.1) | 0.5.1 | transitive | `MIT` | all four |
| [x509-parser](https://crates.io/crates/x509-parser/0.18.1) | 0.18.1 | transitive | `MIT OR Apache-2.0` | all four |
| [xml-no-std](https://crates.io/crates/xml-no-std/0.8.26) | 0.8.26 | transitive | `MIT` | all four |
| [yoke](https://crates.io/crates/yoke/0.8.3) | 0.8.3 | transitive | `Unicode-3.0` | all four |
| [yoke-derive](https://crates.io/crates/yoke-derive/0.8.2) | 0.8.2 | transitive | `Unicode-3.0` | all four |
| [zerocopy](https://crates.io/crates/zerocopy/0.8.56) | 0.8.56 | transitive | `BSD-2-Clause OR Apache-2.0 OR MIT` | all four |
| [zerocopy-derive](https://crates.io/crates/zerocopy-derive/0.8.56) | 0.8.56 | transitive | `BSD-2-Clause OR Apache-2.0 OR MIT` | all four |
| [zerofrom](https://crates.io/crates/zerofrom/0.1.8) | 0.1.8 | transitive | `Unicode-3.0` | all four |
| [zerofrom-derive](https://crates.io/crates/zerofrom-derive/0.1.7) | 0.1.7 | transitive | `Unicode-3.0` | all four |
| [zeroize](https://crates.io/crates/zeroize/1.9.0) | 1.9.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [zeroize_derive](https://crates.io/crates/zeroize_derive/1.5.0) | 1.5.0 | transitive | `Apache-2.0 OR MIT` | all four |
| [zerotrie](https://crates.io/crates/zerotrie/0.2.4) | 0.2.4 | transitive | `Unicode-3.0` | all four |
| [zerovec](https://crates.io/crates/zerovec/0.11.6) | 0.11.6 | transitive | `Unicode-3.0` | all four |
| [zerovec-derive](https://crates.io/crates/zerovec-derive/0.11.3) | 0.11.3 | transitive | `Unicode-3.0` | all four |
| [zip](https://crates.io/crates/zip/8.6.0) | 8.6.0 | transitive | `MIT` | all four |
| [zmij](https://crates.io/crates/zmij/1.0.23) | 1.0.23 | transitive | `MIT` | all four |

### Focused source and license audit

- `c2pa 0.90.12` is the crates.io package with Cargo checksum
  `0bcd2a168e8ce506789d4e5a66c286e5aa4944bc2181d75360b3ddf723ac4264`.
  Its source record pins c2pa-rs revision
  `ae0c3fde8ea399bf7f12379bb44e38b2738b8369`. That tree contains
  `LICENSE-APACHE` and `LICENSE-MIT`, and no `NOTICE`; the packaged crate
  also contains no top-level `NOTICE`.
- `unicode-normalization 0.1.25` is the crates.io package with Cargo checksum
  `5fd4f6878c9cb28d874b009da9e8d183b5abc80117c40bbd187a1fde336be6e8`.
  Its package contains `COPYRIGHT`, `LICENSE-APACHE`, and `LICENSE-MIT`
  and no `NOTICE`.
- `alloc-stdlib 0.2.4` declares `BSD-3-Clause` but its crates.io package
  omits a top-level license file. Its `.cargo_vcs_info.json` identifies
  revision `ae42d22078b98549e987d2f03d12df7b984fde47` of
  `dropbox/rust-alloc-no-stdlib`. The upstream `LICENSE` at that exact
  revision was retrieved and hashed as
  `c0c56f26d9c051cac4d200c34c84e7ae9aaa853e01a982a1df08b09931e518ae`;
  it is byte-identical to the license packaged with
  `alloc-no-stdlib 2.0.4`. The notice is Copyright (c) 2016 Dropbox, Inc.;
  all rights reserved.
- The additional BSD-3-Clause packages preserve these packaged notices:
  `bcder 0.7.7`, Copyright (c) 2018 NLnet Labs;
  `curve25519-dalek 4.1.3`, Copyright (c) 2012 The Go Authors and
  Copyright (c) 2016–2021 isis agora lovecruft and Henry de Valence;
  `ed25519-dalek 2.2.0`, Copyright (c) 2017–2019 isis agora lovecruft;
  and `subtle 2.6.1`, Copyright (c) 2016–2024 Isis Agora Lovecruft and
  Copyright (c) 2016–2017 Isis Agora Lovecruft and Henry de Valence.
  `brotli 7.0.0` and `brotli-decompressor 4.0.3` use the Dropbox
  BSD-3-Clause notice above; `brotli 7.0.0` also declares MIT.
- Unicode-3.0 packages use the Unicode License V3 text bundled in their exact
  crate packages. `unicode-ident 1.0.24` requires that term in addition to
  its MIT-or-Apache choice. The Unicode-only ICU4X support crates are identified
  explicitly in the inventory.
- No package in this 251-package release union exposes a top-level file named
  `NOTICE`. Package metadata and bundled license files remain authoritative.
  Legacy declarations `MIT/Apache-2.0` and `BSD-3-Clause/MIT` are retained
  verbatim rather than reinterpreted.
- Twelve crate packages expose no candidate license file. Their Cargo VCS
  revisions are pinned in `third_party/license-fallbacks/manifest.json`: nine
  revisions provide 17 exact upstream files, while the exact source trees for
  `btree-range-map 0.7.2`, `range-traits 0.3.2`, and
  `static-regular-grammar 2.0.2` provide none. Those three zero-file results are
  recorded explicitly; no generic or different-version text is substituted.

scrub.ts is distributed under Apache-2.0; the full Apache-2.0 text is in the
root `LICENSE`. Every platform archive preserves the exact repository bytes of
that file, this inventory, and `THIRD_PARTY_LICENSES.txt`. This records the
source material actually preserved; it does not interpret compound expressions
or claim legal completeness.

## Committed conformance and replay material

These are not production dependencies, but their adjacent attribution must stay
with the committed corpora.

- Unicode normalization conformance fixtures are derived from the Unicode
  Character Database and are covered by Unicode License V3. The repository
  preserves the license at
  `crates/scrub/tests/fixtures/UNICODE-LICENSE.txt` and records source
  revisions and checksums in `research/sources.yaml`.
- C2PA fixtures adapted from c2pa-rs remain under its MIT OR Apache-2.0 terms;
  adjacent `LICENSE-APACHE` and `LICENSE-MIT` files are committed under
  `crates/scrub/tests/fixtures/c2pa/c2pa-rs/`.
- The two C2PA public-testfiles fixtures retain their CC-BY-SA-4.0 attribution
  and license under
  `crates/scrub/tests/fixtures/c2pa/public-testfiles/`.
- Repository-generated hostile and controlled fixtures record their exact
  construction and source status in adjacent README files and
  `research/sources.yaml`.

## WaterLARP v1 Python dependencies

WaterLARP is a separate research package and is not part of the native scrub
release dependency inventory above. Exact direct versions and the resolved
Windows/CPython 3.13 environment are frozen in `waterlarp/pyproject.toml` and
`waterlarp/requirements-lock.txt`. No upstream Python source is copied into
the repository. Adapters load installed packages or disposable pinned checkouts
at execution time.

| Component | Version | License | Reason / maintenance judgment |
|---|---:|---|---|
| PyTorch | 2.10.0 CPU | BSD-3-Clause | Required tensor/RNG/model execution substrate used by both official implementations; mature, actively maintained |
| Transformers | 5.15.0 | Apache-2.0 | Official released SynthID generation integration and open-weight model API; pinned tag/commit and parity vector |
| datasets | 5.0.1 | Apache-2.0 | Revision-aware dataset/cache boundary; PILOT used bounded dataset-server rows after C4 builder proved unbounded for this host |
| NumPy | 2.3.1 | BSD-3-Clause | Vectorized entropy and aggregation math |
| SciPy | 1.17.1 | BSD-3-Clause | Exact beta quantiles for Clopper-Pearson intervals and author-reference KGW p-values |
| tokenizers | 0.22.2 | Apache-2.0 | Newest released wheel compatible with Transformers 5.15.0 on this interpreter |
| Pydantic | 2.11.7 | MIT | Explicit schema-validation dependency reserved for config/result expansion |
| PyYAML | 6.0.3 | MIT | Research ledger and human-authored config parsing |
| tqdm | 4.67.1 | MPL-2.0 OR MIT | Progress reporting from ML/data dependencies; diagnostics do not enter machine JSON stdout |

Development-only validation uses jsonschema 4.25.1 (MIT) to execute the public
Draft 2020-12 result schemas. It is not imported by canonical experiment or
aggregation runtime code.

Resolved transitive licenses were reviewed from installed package metadata.
They are predominantly Apache-2.0, BSD, MIT, PSF, ISC, or MPL-2.0. Notable
packages include PyArrow (Apache-2.0), pandas (BSD-3-Clause), Hugging Face Hub
(Apache-2.0), safetensors (Apache-2.0), requests (Apache-2.0), and aiohttp
(Apache-2.0 OR MIT). Packaged license files remain authoritative.

KGW author code (Apache-2.0), DeepMind SynthID reference code (Apache-2.0), and
Transformers (Apache-2.0) are used through adapters/parity checkouts; none is
copied. WaterSeeker code was not adapted because its pinned repository exposed
no license file. Raw C4 (ODC-BY with source-specific terms), GSM8K (MIT), MBPP
(CC-BY-4.0), model weights, and bounded dataset rows remain ignored caches and
are not tracked as fixtures.
