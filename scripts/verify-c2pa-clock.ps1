param(
    [string]$CargoTargetDir = (Join-Path $env:TEMP 'scrubts-c2pa-clock-target')
)

$ErrorActionPreference = 'Stop'
$expectedCrateSha256 = '0bcd2a168e8ce506789d4e5a66c286e5aa4944bc2181d75360b3ddf723ac4264'
$archive = Get-ChildItem -Path (Join-Path $env:USERPROFILE '.cargo\registry\cache') -Recurse -Filter 'c2pa-0.90.12.crate' |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $archive) {
    throw 'the cached c2pa 0.90.12 crate archive is unavailable'
}
$actualCrateSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
if ($actualCrateSha256 -ne $expectedCrateSha256) {
    throw "unexpected c2pa 0.90.12 archive SHA-256: $actualCrateSha256"
}

$probeRoot = Join-Path $env:TEMP ("scrubts-c2pa-clock-probe-{0}" -f [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $probeRoot | Out-Null
try {
    tar -xf $archive -C $probeRoot
    if ($LASTEXITCODE -ne 0) {
        throw "could not extract $archive"
    }
    $sourceRoot = Join-Path $probeRoot 'c2pa-0.90.12'
    $manifest = Join-Path $sourceRoot 'Cargo.toml'
    if (-not (Select-String -LiteralPath $manifest -Pattern '^version = "0\.90\.12"$' -Quiet)) {
        throw 'extracted source is not c2pa 0.90.12'
    }

    $repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
    $fixtureRoot = Join-Path $repositoryRoot 'crates\scrub\tests\fixtures\c2pa'
    $fixtureDir = Join-Path $sourceRoot 'scrub-clock-fixtures'
    New-Item -ItemType Directory -Path $fixtureDir | Out-Null
    $fixtures = @(
        @{
            Source = Join-Path $fixtureRoot 'generated\signed.png'
            Destination = Join-Path $fixtureDir 'signed.png'
            Length = 312595
            Sha256 = '276e64f0ba1f0ed3cd153f5fb166fb1864fadd03fd6d3cd5427cc77fc935fdb0'
        },
        @{
            Source = Join-Path $fixtureRoot 'c2pa-rs\ocsp_with_assertion.jpg'
            Destination = Join-Path $fixtureDir 'ocsp_with_assertion.jpg'
            Length = 599791
            Sha256 = '210fb95c6a766d3cd89ef0583898ec7248fe60f0ed651af216fb270cd9cbe17a'
        }
    )
    foreach ($fixture in $fixtures) {
        $item = Get-Item -LiteralPath $fixture.Source
        $sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $fixture.Source).Hash.ToLowerInvariant()
        if ($item.Length -ne $fixture.Length -or $sha256 -ne $fixture.Sha256) {
            throw "unexpected clock fixture identity: $($fixture.Source)"
        }
        Copy-Item -LiteralPath $fixture.Source -Destination $fixture.Destination
    }

    $utf8 = New-Object Text.UTF8Encoding($false)
    $timePath = Join-Path $sourceRoot 'src\crypto\internal\time.rs'
    $timeSource = [IO.File]::ReadAllText($timePath)
    $timeNeedle = @'
use chrono::{DateTime, Utc};

/// Return the current time in UTC.
pub(crate) fn utc_now() -> DateTime<Utc> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Utc::now()
    }
'@
    $timeReplacement = @'
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicI64, Ordering};

static SCRUB_TEST_UTC_NOW: AtomicI64 = AtomicI64::new(i64::MIN);

pub(crate) fn set_utc_now_for_scrub_test(epoch: i64) {
    SCRUB_TEST_UTC_NOW.store(epoch, Ordering::SeqCst);
}

/// Return the current time in UTC.
pub(crate) fn utc_now() -> DateTime<Utc> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let epoch = SCRUB_TEST_UTC_NOW.load(Ordering::SeqCst);
        if epoch != i64::MIN {
            return DateTime::from_timestamp(epoch, 0).expect("controlled test time is valid");
        }
        Utc::now()
    }
'@
    if (-not $timeSource.Contains($timeNeedle)) {
        throw 'pinned time source no longer matches the probe patch point'
    }
    [IO.File]::WriteAllText($timePath, $timeSource.Replace($timeNeedle, $timeReplacement), $utf8)

    $certificatePath = Join-Path $sourceRoot 'src\crypto\cose\certificate_profile.rs'
    $certificateSource = [IO.File]::ReadAllText($certificatePath)
    $certificateSource = $certificateSource.Replace("use web_time::SystemTime;`n", '')
    $certificateNeedle = @'
        let Ok(now) = SystemTime::now().duration_since(web_time::UNIX_EPOCH) else {
            return Err(CertificateProfileError::InternalError(
                "system time invalid".to_string(),
            ));
        };

        if !signcert.validity().is_valid_at(
            x509_parser::time::ASN1Time::from_timestamp(now.as_secs() as i64)
'@
    $certificateReplacement = @'
        let now = crate::crypto::internal::time::utc_now().timestamp();

        if !signcert.validity().is_valid_at(
            x509_parser::time::ASN1Time::from_timestamp(now)
'@
    if (-not $certificateSource.Contains($certificateNeedle)) {
        throw 'pinned certificate source no longer matches the probe patch point'
    }
    [IO.File]::WriteAllText(
        $certificatePath,
        $certificateSource.Replace($certificateNeedle, $certificateReplacement),
        $utf8
    )

    $libPath = Join-Path $sourceRoot 'src\lib.rs'
    $libSource = [IO.File]::ReadAllText($libPath)
    $probe = @'

#[doc(hidden)]
pub fn scrub_controlled_clock_probe() {
    use std::io::Cursor;
    use chrono::{TimeZone as _, Utc};

    fn contains(bytes: &[u8], needle: &[u8]) -> bool {
        bytes.windows(needle.len()).any(|window| window == needle)
    }

    fn codes_at(bytes: &'static [u8], format: &str, epoch: i64) -> Vec<String> {
        crate::crypto::internal::time::set_utc_now_for_scrub_test(epoch);
        let context = Context::new()
            .with_settings(r#"{
                "core": {"allowed_network_hosts": [], "decode_identity_assertions": false},
                "verify": {
                    "verify_after_reading": true,
                    "verify_trust": false,
                    "verify_timestamp_trust": false,
                    "ocsp_fetch": false,
                    "remote_manifest_fetch": false
                }
            }"#)
            .expect("controlled settings are valid");
        Reader::from_context(context)
            .with_stream(format, Cursor::new(bytes))
            .expect("pinned fixture is readable")
            .validation_results()
            .expect("validation results exist")
            .validation_status()
            .into_iter()
            .map(|status| status.code().to_owned())
            .collect()
    }

    let certificate = include_bytes!("../scrub-clock-fixtures/signed.png");
    assert!(contains(certificate, b"220610184641Z"));
    assert!(contains(certificate, b"300826184641Z"));
    let certificate_before = codes_at(
        certificate,
        "image/png",
        Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap().timestamp(),
    );
    let certificate_inside = codes_at(
        certificate,
        "image/png",
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().timestamp(),
    );
    eprintln!("certificate_before={certificate_before:?}");
    eprintln!("certificate_inside={certificate_inside:?}");
    assert!(certificate_before.iter().any(|code| code == validation_status::SIGNING_CREDENTIAL_EXPIRED));
    assert!(!certificate_inside.iter().any(|code| code == validation_status::SIGNING_CREDENTIAL_EXPIRED));
    assert!(certificate_inside.iter().any(|code| code == validation_status::CLAIM_SIGNATURE_VALIDATED));

    fn certificate_status_at(bytes: &[u8], epoch: i64) -> (bool, bool) {
        use crate::{
            assertion::AssertionData,
            crypto::{cose::cert_chain_from_sign1, ocsp::OcspResponse},
            status_tracker::StatusTracker,
            store::Store,
        };

        crate::crypto::internal::time::set_utc_now_for_scrub_test(epoch);
        let context = Context::new()
            .with_settings(r#"{
                "core": {"allowed_network_hosts": [], "decode_identity_assertions": false},
                "verify": {"verify_after_reading": false, "ocsp_fetch": false, "remote_manifest_fetch": false}
            }"#)
            .expect("controlled settings are valid");
        let jumbf = crate::jumbf_io::load_jumbf_from_memory("image/jpeg", bytes)
            .expect("fixture JUMBF is available");
        let mut parse_log = StatusTracker::default();
        let store = Store::from_jumbf_with_context(&jumbf, &mut parse_log, &context)
            .expect("fixture store parses");
        let claim = store
            .claims()
            .into_iter()
            .find(|claim| !claim.certificate_status_assertions().is_empty())
            .expect("fixture has a CertificateStatus claim");
        let assertion = claim.certificate_status_assertions()[0].assertion();
        let AssertionData::Cbor(cbor) = assertion.decode_data() else {
            panic!("CertificateStatus is not CBOR");
        };
        let value: c2pa_cbor::Value = c2pa_cbor::from_slice(cbor.as_slice())
            .expect("CertificateStatus raw CBOR decodes");
        let responses = value
            .as_map()
            .and_then(|map| {
                map.iter().find_map(|(key, value)| {
                    (key.as_str() == Some("ocspVals")).then(|| value.as_array()).flatten()
                })
            })
            .expect("CertificateStatus has ocspVals");
        let mut status_log = StatusTracker::default();
        for response in responses {
            for candidate in store.claims() {
                let chain = cert_chain_from_sign1(&candidate.cose_sign1().expect("claim has COSE"))
                    .expect("COSE has a certificate chain");
                OcspResponse::from_der_checked(
                    response.as_bytes().expect("OCSP value is bytes"),
                    &chain,
                    None,
                    &mut status_log,
                )
                    .expect("OCSP response processing completes");
            }
        }
        (
            status_log.has_status(validation_status::SIGNING_CREDENTIAL_REVOKED),
            status_log.has_status(validation_status::SIGNING_CREDENTIAL_NOT_REVOKED),
        )
    }

    let certificate_status = include_bytes!("../scrub-clock-fixtures/ocsp_with_assertion.jpg");
    assert!(contains(certificate_status, b"20250716182311Z"));
    assert!(contains(certificate_status, b"20250723182311Z"));
    let status_before = certificate_status_at(
        certificate_status,
        Utc.with_ymd_and_hms(2025, 7, 15, 0, 0, 0).unwrap().timestamp(),
    );
    let status_after = certificate_status_at(
        certificate_status,
        Utc.with_ymd_and_hms(2025, 7, 17, 0, 0, 0).unwrap().timestamp(),
    );
    eprintln!("certificate_status_before={status_before:?}");
    eprintln!("certificate_status_after={status_after:?}");
    assert_eq!(status_before, (true, false));
    assert_eq!(status_after, (false, true));
}
'@
    [IO.File]::WriteAllText($libPath, $libSource + $probe, $utf8)

    $harnessRoot = Join-Path $probeRoot 'harness'
    New-Item -ItemType Directory -Path (Join-Path $harnessRoot 'src') -Force | Out-Null
    $harnessManifest = @'
[package]
name = "scrub-c2pa-clock-probe"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
c2pa = { path = "../c2pa-0.90.12", default-features = false, features = ["rust_native_crypto"] }
'@
    [IO.File]::WriteAllText((Join-Path $harnessRoot 'Cargo.toml'), $harnessManifest, $utf8)
    [IO.File]::WriteAllText(
        (Join-Path $harnessRoot 'src\main.rs'),
        "fn main() { c2pa::scrub_controlled_clock_probe(); }`n",
        $utf8
    )
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'Cargo.lock') -Destination (Join-Path $harnessRoot 'Cargo.lock')

    $env:CARGO_TARGET_DIR = $CargoTargetDir
    cargo run --manifest-path (Join-Path $harnessRoot 'Cargo.toml') --offline
    if ($LASTEXITCODE -ne 0) {
        throw 'controlled-time c2pa 0.90.12 probe failed'
    }
}
finally {
    if (Test-Path -LiteralPath $probeRoot) {
        $resolvedProbeRoot = (Resolve-Path -LiteralPath $probeRoot).Path
        $resolvedTemp = (Resolve-Path -LiteralPath $env:TEMP).Path
        if (-not $resolvedProbeRoot.StartsWith($resolvedTemp + [IO.Path]::DirectorySeparatorChar)) {
            throw "refusing to remove unexpected probe path: $resolvedProbeRoot"
        }
        Remove-Item -Recurse -Force -LiteralPath $resolvedProbeRoot
    }
}
