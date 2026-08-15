# Third-party notices

This ledger is for third-party software or adapted code actually included in scrub.ts. Projects that merely influenced methodology belong in `research/sources.yaml` instead.

For each included/adapted component record:

- component + upstream URL;
- pinned version/commit;
- upstream authors/organization as required;
- license;
- integration mode (dependency, adapter, adapted code, generated data);
- local files affected;
- modifications;
- required copyright/license/NOTICE text.

## Milestone 1 Rust dependencies

The following crates are unmodified Cargo dependencies. Exact versions are
pinned in `Cargo.lock`. No upstream code was copied or adapted into repository
source files.

| Component | Upstream | Version | License | Integration / local files |
|---|---|---:|---|---|
| serde | https://github.com/serde-rs/serde | 1.0.229 | MIT OR Apache-2.0 | direct dependency and derive macros; `crates/scrub-report/Cargo.toml`, `Cargo.lock` |
| serde_json | https://github.com/serde-rs/json | 1.0.151 | MIT OR Apache-2.0 | direct JSON dependency; `crates/scrub-report/Cargo.toml`, `Cargo.lock` |
| sha2 | https://github.com/RustCrypto/hashes | 0.11.0 | MIT OR Apache-2.0 | direct SHA-256 dependency; `crates/scrub/Cargo.toml`, `Cargo.lock` |

Resolved transitive dependencies were checked against their packaged Cargo
metadata in the local toolchain environment:

| Component | Version | License |
|---|---:|---|
| block-buffer | 0.12.1 | MIT OR Apache-2.0 |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 |
| const-oid | 0.10.2 | Apache-2.0 OR MIT |
| cpufeatures | 0.3.0 | MIT OR Apache-2.0 |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 |
| digest | 0.11.3 | MIT OR Apache-2.0 |
| hybrid-array | 0.4.14 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| libc | 0.2.189 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| syn | 3.0.3 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| zmij | 1.0.23 | MIT |

The dependency crates' packaged license files remain authoritative. The local
package sources for these exact versions contain no top-level file named
`NOTICE`. Binary/source redistribution must continue to satisfy the selected
license terms and preserve required license text.

## WaterLARP v1 Python dependencies

WaterLARP is a separate research package. Exact direct versions and the full
resolved Windows/CPython 3.13 environment are frozen in
`waterlarp/pyproject.toml` and `waterlarp/requirements-lock.txt`. No upstream
Python source is copied into the repository. Adapters load installed packages
or disposable pinned checkouts at execution time.

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
