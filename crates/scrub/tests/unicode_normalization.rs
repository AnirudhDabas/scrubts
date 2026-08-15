#[path = "support/unicode_normalization_corpus.rs"]
mod corpus;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use corpus::{ExpectedFinding, INSPECTION_READ_BOUNDARY, fixture_corpus};
use scrub_report::{Evidence, Finding, FindingStatus, Report, Sha256Digest};
use sha2::{Digest, Sha256};

const NFC_MECHANISM_ID: &str = "unicode.normalization.nfc_difference";
const NFKC_MECHANISM_ID: &str = "unicode.normalization.nfkc_difference";
const UNICODE_VERSION: &str = "17.0.0";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempArtifact {
    directory: PathBuf,
    path: PathBuf,
}

impl TempArtifact {
    fn new(contents: &[u8]) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "scrub-unicode-normalization-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("temporary directory can be created");
        let path = directory.join("artifact.txt");
        fs::write(&path, contents).expect("temporary artifact can be written");
        Self { directory, path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempArtifact {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.directory);
    }
}

#[test]
fn compiled_cli_matches_the_complete_scrub_specific_corpus() {
    let fixtures = fixture_corpus();
    assert_eq!(fixtures.len(), 22);

    for fixture in fixtures {
        let artifact = TempArtifact::new(&fixture.input);
        let before = fs::read(artifact.path()).expect("artifact can be read before inspection");
        let output = inspect(artifact.path(), true);
        assert!(
            output.status.success(),
            "{} stderr: {}",
            fixture.name,
            stderr(&output)
        );
        assert_eq!(stderr(&output), "", "{} stderr", fixture.name);
        let json = stdout(&output)
            .strip_suffix('\n')
            .expect("JSON output has one trailing newline");
        assert!(!json.contains('\n'), "{} JSON is one line", fixture.name);
        let report = Report::from_json(json)
            .expect("stdout is an untrusted report")
            .into_report();
        assert_eq!(
            report.findings().len(),
            10,
            "{} finding count",
            fixture.name
        );
        assert_eq!(
            report
                .findings()
                .iter()
                .map(|finding| finding.mechanism().id())
                .collect::<Vec<_>>(),
            [
                "unicode.bidi_control",
                "unicode.default_ignorable_code_point",
                NFC_MECHANISM_ID,
                NFKC_MECHANISM_ID,
                "c2pa.text_manifest_wrapper",
                "c2pa.manifest_store",
                "c2pa.manifest_validation",
                "c2pa.hard_binding",
                "c2pa.credential_trust",
                "anthropic.embedded_text_watermark",
            ],
            "{} finding order",
            fixture.name
        );

        assert_expected(
            finding(&report, NFC_MECHANISM_ID),
            &fixture.nfc,
            fixture.name,
        );
        assert_expected(
            finding(&report, NFKC_MECHANISM_ID),
            &fixture.nfkc,
            fixture.name,
        );
        assert_eq!(
            report.artifact().byte_length(),
            u64::try_from(fixture.input.len()).expect("fixture length fits u64")
        );
        let expected_digest: [u8; 32] = Sha256::digest(&fixture.input).into();
        assert_eq!(
            report.artifact().content_sha256(),
            &Sha256Digest::from_bytes(expected_digest)
        );
        match fixture.name {
            "valid_multibyte_scalar_crosses_real_65536_boundary" => {
                assert_frozen_artifact_identity(
                    &report,
                    65_537,
                    "0e98edc686f385514eef3579539b707bb69bd41919ddd6a3b035e9fbd6a6b8a7",
                );
            }
            "normalization_sequence_crosses_real_65536_boundary" => {
                assert_frozen_artifact_identity(
                    &report,
                    65_538,
                    "e485b9462a538d359eb8f48cd9547eb6039e66d2e7984676cd2e8f468661f910",
                );
            }
            _ => {}
        }
        assert_eq!(
            fs::read(artifact.path()).expect("artifact can be read after inspection"),
            before,
            "{} input changed",
            fixture.name
        );
    }
}

#[test]
fn raw_json_and_human_output_freeze_public_order_and_neutral_wording() {
    let artifact = TempArtifact::new("\u{754c}e\u{301}\u{fb03}".as_bytes());
    let json_output = inspect(artifact.path(), true);
    assert!(
        json_output.status.success(),
        "stderr: {}",
        stderr(&json_output)
    );
    let raw = stdout(&json_output);
    let positions: Vec<_> = [
        "unicode.bidi_control",
        "unicode.default_ignorable_code_point",
        NFC_MECHANISM_ID,
        NFKC_MECHANISM_ID,
    ]
    .into_iter()
    .map(|needle| {
        raw.find(needle)
            .unwrap_or_else(|| panic!("{needle} exists"))
    })
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    let nfc_start = raw.find(NFC_MECHANISM_ID).expect("NFC finding exists");
    let nfkc_start = raw.find(NFKC_MECHANISM_ID).expect("NFKC finding exists");
    for finding_json in [&raw[nfc_start..nfkc_start], &raw[nfkc_start..]] {
        let evidence_positions: Vec<_> = [
            "first_difference",
            "normalized_byte_length",
            "normalized_scalar_count",
            "normalized_sha256",
        ]
        .into_iter()
        .map(|needle| {
            finding_json
                .find(needle)
                .unwrap_or_else(|| panic!("{needle} exists"))
        })
        .collect();
        assert!(evidence_positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    let human_output = inspect(artifact.path(), false);
    assert!(
        human_output.status.success(),
        "stderr: {}",
        stderr(&human_output)
    );
    let human = stdout(&human_output);
    assert!(human.contains("PRESENT         NFC difference"));
    assert!(human.contains("PRESENT         NFKC difference"));
    assert!(human.contains("first_difference  {\"scalar_index\":1"));
    assert!(!human.contains('\u{301}'));
    assert!(!human.contains('\u{fb03}'));
    let lower = human.to_ascii_lowercase();
    for prohibited in [
        "malicious",
        "suspicious",
        "watermark detected",
        "watermark removed",
        "ai-generated",
        "sanitized",
        "safe",
    ] {
        assert!(
            !lower.contains(prohibited),
            "prohibited wording {prohibited}"
        );
    }
}

#[test]
fn complete_human_output_is_frozen_for_a_canonical_difference() {
    let artifact = TempArtifact::new("e\u{301}".as_bytes());
    let output = inspect(artifact.path(), false);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "scrub inspect\n",
            "\n",
            "Artifact\n",
            "  name    artifact.txt\n",
            "  size    3 bytes\n",
            "  sha256  bf12767b0f2a56b2190075bae8169f656e3ce8d6357d4aff184bc6c7ea48f9f6\n",
            "\n",
            "Observations\n",
            "\n",
            "  Unicode\n",
            "    PRESENT         NFC difference\n",
            "      first_difference  {\"scalar_index\":0,\"original_byte_offset\":0,\"original\":{\"at_end\":false,\"scalars\":[\"U+0065\",\"U+0301\"],\"truncated\":false},\"normalized\":{\"at_end\":false,\"scalars\":[\"U+00E9\"],\"truncated\":false}}\n",
            "      normalized_byte_length  2\n",
            "    PRESENT         NFKC difference\n",
            "      first_difference  {\"scalar_index\":0,\"original_byte_offset\":0,\"original\":{\"at_end\":false,\"scalars\":[\"U+0065\",\"U+0301\"],\"truncated\":false},\"normalized\":{\"at_end\":false,\"scalars\":[\"U+00E9\"],\"truncated\":false}}\n",
            "      normalized_byte_length  2\n",
            "\n",
            "  C2PA\n",
            "    ABSENT          text manifest wrapper\n",
            "\n",
            "  Claude\n",
            "    UNKNOWN         embedded text watermark\n",
            "      verifier          anthropic.provider_detector\n",
            "                        unavailable in checked authority snapshot\n",
            "      reference         reference.synthid_text\n",
            "                        related family; not deployment parity\n",
            "      supports          mechanism family disclosed; provider detector unavailable\n",
            "      does not support  Claude watermark presence/absence or provider parity\n",
            "\n",
            "Interpretation\n",
            "  A Unicode PRESENT finding supports only its reported Unicode observation.\n",
            "  It does not establish an Anthropic watermark, Claude involvement, or authorship.\n",
            "  UNKNOWN != ABSENT / CLEAN.\n",
            "  No aggregate authorship or artifact-clean verdict is reported.\n",
            "  Use --explain for the complete evidence and authority chain.\n",
        )
    );
}

#[test]
fn malformed_utf8_variants_are_invalid_without_normalized_prefix_evidence() {
    let variants: Vec<(&str, Vec<u8>)> = vec![
        ("lone_continuation", vec![0x80]),
        ("overlong", vec![0xc0, 0xaf]),
        ("surrogate", vec![0xed, 0xa0, 0x80]),
        ("above_maximum", vec![0xf4, 0x90, 0x80, 0x80]),
        ("truncated_eof", vec![0xe2, 0x80]),
        ("sensitive_prefix_then_malformed", {
            let mut input = "e\u{301}".as_bytes().to_vec();
            input.push(0xff);
            input
        }),
        (
            "malformed_crossing_real_boundary",
            malformed_boundary_input(),
        ),
        ("late_malformed_with_trailing_bytes", late_malformed_input()),
    ];

    for (name, input) in variants {
        let artifact = TempArtifact::new(&input);
        let output = inspect(artifact.path(), true);
        assert!(
            output.status.success(),
            "{name} stderr: {}",
            stderr(&output)
        );
        let report = Report::from_json(stdout(&output).trim_end())
            .expect("stdout is an untrusted report")
            .into_report();
        assert_eq!(
            report.artifact().byte_length(),
            u64::try_from(input.len()).expect("fixture length fits u64"),
            "{name} length"
        );
        let expected_digest: [u8; 32] = Sha256::digest(&input).into();
        assert_eq!(
            report.artifact().content_sha256(),
            &Sha256Digest::from_bytes(expected_digest),
            "{name} digest"
        );
        match name {
            "malformed_crossing_real_boundary" => assert_frozen_artifact_identity(
                &report,
                131_074,
                "bcd43a095b1db13232c50b8b94426e4131a4a276190f8eb8611c05a911dbbb91",
            ),
            "late_malformed_with_trailing_bytes" => assert_frozen_artifact_identity(
                &report,
                196_634,
                "157db501641a7d3794be72c720cb9e4b69f778aa11a746cf9e94d7a8c27469fa",
            ),
            _ => {}
        }
        for mechanism_id in [NFC_MECHANISM_ID, NFKC_MECHANISM_ID] {
            let finding = finding(&report, mechanism_id);
            assert_eq!(finding.status(), FindingStatus::Invalid, "{name} status");
            assert_eq!(
                finding.evidence(),
                [Evidence::new(
                    "utf8_validation",
                    "failed: the complete artifact is malformed or incomplete UTF-8",
                )],
                "{name} evidence"
            );
            for absent in [
                "first_difference",
                "normalized_byte_length",
                "normalized_scalar_count",
                "normalized_sha256",
            ] {
                assert!(finding.evidence().iter().all(|item| item.name() != absent));
            }
        }
        assert_eq!(
            fs::read(artifact.path()).expect("artifact remains readable"),
            input,
            "{name} input changed"
        );
    }
}

#[test]
fn repeated_json_and_human_executions_are_byte_identical_and_preserve_input() {
    let fixture = fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == "back_to_back_sensitive_sequences")
        .expect("determinism fixture exists");
    let artifact = TempArtifact::new(&fixture.input);
    let before = fs::read(artifact.path()).expect("artifact can be read");
    for json in [true, false] {
        let first = inspect(artifact.path(), json);
        assert!(first.status.success(), "stderr: {}", stderr(&first));
        for run in 2..=8 {
            let next = inspect(artifact.path(), json);
            assert!(next.status.success(), "run {run} stderr: {}", stderr(&next));
            assert_eq!(next.stdout, first.stdout, "run {run} stdout");
            assert_eq!(next.stderr, first.stderr, "run {run} stderr");
        }
    }
    assert_eq!(
        fs::read(artifact.path()).expect("artifact can be read"),
        before
    );
}

fn malformed_boundary_input() -> Vec<u8> {
    let mut input = vec![b'a'; INSPECTION_READ_BOUNDARY - 1];
    input.extend_from_slice(&[0xe2, 0x28, 0xa1]);
    input.extend(std::iter::repeat_n(b'z', INSPECTION_READ_BOUNDARY));
    input
}

fn late_malformed_input() -> Vec<u8> {
    let mut input = vec![b'a'; INSPECTION_READ_BOUNDARY * 3 + 17];
    input.extend_from_slice("e\u{301}".as_bytes());
    input.push(0xff);
    input.extend_from_slice(&[0, 1, 2, 0xfe, b'z']);
    input
}

fn assert_expected(finding: &Finding, expected: &ExpectedFinding, context: &str) {
    assert_eq!(finding.mechanism().version(), UNICODE_VERSION, "{context}");
    assert_eq!(finding.status(), expected.status(), "{context} status");
    assert_eq!(
        finding.evidence(),
        expected.evidence(),
        "{context} evidence"
    );
}

fn assert_frozen_artifact_identity(report: &Report, byte_length: u64, sha256: &str) {
    assert_eq!(report.artifact().byte_length(), byte_length);
    assert_eq!(report.artifact().content_sha256().to_string(), sha256);
}

fn finding<'a>(report: &'a Report, mechanism_id: &str) -> &'a Finding {
    report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == mechanism_id)
        .unwrap_or_else(|| panic!("finding {mechanism_id} exists"))
}

fn inspect(path: &Path, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrub"));
    command.arg("inspect").arg(path);
    if json {
        command.arg("--json");
    }
    command.output().expect("scrub process can run")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr is UTF-8")
}
