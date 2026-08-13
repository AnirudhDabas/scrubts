#[path = "support/unicode_bidi_control.rs"]
mod unicode_bidi_control;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::{Evidence, Finding, FindingStatus, Report, Sha256Digest};
use sha2::{Digest, Sha256};
use unicode_bidi_control::{
    CodePointRange, ExpectedObservation, Fixture, INSPECTION_READ_BOUNDARY, control_identities,
    fixture_corpus, parse_bidi_control_ranges, pinned_bidi_control_ranges,
};

const BIDI_MECHANISM_ID: &str = "unicode.bidi_control";
const DICP_MECHANISM_ID: &str = "unicode.default_ignorable_code_point";
const NFC_MECHANISM_ID: &str = "unicode.normalization.nfc_difference";
const NFKC_MECHANISM_ID: &str = "unicode.normalization.nfkc_difference";
const UNICODE_VERSION: &str = "17.0.0";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn assert_frozen_oracle_is_available_to_compiled_path_tests() {
    let ranges = pinned_bidi_control_ranges();
    assert_eq!(ranges.len(), 4);
    assert_eq!(
        ranges
            .iter()
            .copied()
            .map(CodePointRange::code_point_count)
            .sum::<u32>(),
        12
    );
    assert!(ranges.iter().copied().any(|range| range.contains(0x202E)));
    assert!(!ranges.iter().copied().any(|range| range.contains(0x200D)));
    assert_eq!(control_identities().len(), 12);
    assert_eq!(
        parse_bidi_control_ranges("202E ; Bidi_Control").expect("test property record parses"),
        [CodePointRange {
            start: 0x202E,
            end: 0x202E,
        }]
    );
    for fixture in fixture_corpus() {
        assert_eq!(
            fixture.expected.property_evidence(),
            match &fixture.expected {
                ExpectedObservation::Valid(expected) => expected.report_evidence(),
                ExpectedObservation::InvalidUtf8 => vec![],
            }
        );
    }
}

struct TempArtifact {
    directory: PathBuf,
    path: PathBuf,
}

impl TempArtifact {
    fn new(contents: &[u8]) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "scrub-unicode-bidi-test-{}-{id}",
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
fn real_inspection_path_matches_all_35_frozen_fixtures() {
    assert_frozen_oracle_is_available_to_compiled_path_tests();
    let fixtures = fixture_corpus();
    assert_eq!(fixtures.len(), 35);

    for fixture in fixtures {
        let artifact = TempArtifact::new(&fixture.input);
        let before = fs::read(artifact.path()).expect("artifact can be read before inspection");
        let output = inspect(artifact.path());
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
        let report = Report::from_json(json).expect("stdout is a report");
        assert_eq!(
            report.to_json().expect("report reserializes"),
            json,
            "{} stdout is canonical",
            fixture.name
        );
        assert_eq!(report.findings().len(), 4, "{} finding count", fixture.name);
        assert_eq!(
            report
                .findings()
                .iter()
                .map(|finding| finding.mechanism().id())
                .collect::<Vec<_>>(),
            [
                BIDI_MECHANISM_ID,
                DICP_MECHANISM_ID,
                NFC_MECHANISM_ID,
                NFKC_MECHANISM_ID,
            ],
            "{} finding order",
            fixture.name
        );

        let bidi_finding = finding(&report, BIDI_MECHANISM_ID);
        assert_eq!(
            bidi_finding.mechanism().version(),
            UNICODE_VERSION,
            "{} version",
            fixture.name
        );
        assert_eq!(
            bidi_finding.status(),
            fixture.expected.status(),
            "{} status",
            fixture.name
        );
        match &fixture.expected {
            ExpectedObservation::Valid(expected) => {
                assert_eq!(
                    bidi_finding.evidence(),
                    expected.report_evidence(),
                    "{} evidence",
                    fixture.name
                );
            }
            ExpectedObservation::InvalidUtf8 => {
                assert_invalid_without_property_evidence(bidi_finding, fixture.name);
                assert_invalid_without_property_evidence(
                    finding(&report, DICP_MECHANISM_ID),
                    fixture.name,
                );
            }
        }

        let expected_digest: [u8; 32] = Sha256::digest(&fixture.input).into();
        assert_eq!(
            report.artifact().byte_length(),
            u64::try_from(fixture.input.len()).expect("fixture length fits u64"),
            "{} byte length",
            fixture.name
        );
        assert_eq!(
            report.artifact().content_sha256().as_str(),
            Sha256Digest::from_bytes(expected_digest).as_str(),
            "{} SHA-256",
            fixture.name
        );
        if let Some(frozen) = fixture.frozen_artifact_identity {
            assert_eq!(
                report.artifact().byte_length(),
                frozen.byte_length,
                "{} frozen length",
                fixture.name
            );
            assert_eq!(
                report.artifact().content_sha256().as_str(),
                frozen.sha256,
                "{} frozen SHA-256",
                fixture.name
            );
        }

        let after = fs::read(artifact.path()).expect("artifact can be read after inspection");
        assert_eq!(after, before, "{} input changed", fixture.name);
    }
}

#[test]
fn compiled_path_preserves_independent_overlap_and_zwj_separation() {
    let zwj_report = inspect_fixture("dicp_non_bidi_zwj").1;
    assert_eq!(
        finding(&zwj_report, BIDI_MECHANISM_ID).status(),
        FindingStatus::Absent
    );
    assert_eq!(
        finding(&zwj_report, DICP_MECHANISM_ID).status(),
        FindingStatus::Present
    );

    let all_report = inspect_fixture("all_control_identities").1;
    let bidi = finding(&all_report, BIDI_MECHANISM_ID);
    let dicp = finding(&all_report, DICP_MECHANISM_ID);
    assert_eq!(bidi.status(), FindingStatus::Present);
    assert_eq!(dicp.status(), FindingStatus::Present);
    assert_eq!(evidence_value(bidi, "total_occurrence_count"), "12");
    assert_eq!(evidence_value(dicp, "total_occurrence_count"), "12");
    for abbreviation in [
        "ALM", "LRM", "RLM", "LRE", "RLE", "PDF", "LRO", "RLO", "LRI", "RLI", "FSI", "PDI",
    ] {
        assert!(
            evidence_value(bidi, "locations")
                .contains(&format!("\"abbreviation\":\"{abbreviation}\""))
        );
    }
}

#[test]
fn compiled_path_retains_the_first_256_and_never_replaces_them() {
    let report_256 = inspect_fixture("exactly_256_bidi_controls").1;
    let report_257 = inspect_fixture("exactly_257_bidi_controls").1;
    let finding_256 = finding(&report_256, BIDI_MECHANISM_ID);
    let finding_257 = finding(&report_257, BIDI_MECHANISM_ID);

    assert_eq!(evidence_value(finding_256, "total_occurrence_count"), "256");
    assert_eq!(evidence_value(finding_256, "locations_truncated"), "false");
    assert_eq!(evidence_value(finding_257, "total_occurrence_count"), "257");
    assert_eq!(evidence_value(finding_257, "locations_truncated"), "true");
    assert_eq!(
        evidence_value(finding_257, "locations"),
        evidence_value(finding_256, "locations")
    );
    assert!(evidence_value(finding_257, "locations").ends_with(
        r#"{"code_point":"U+202E","abbreviation":"RLO","byte_offset":765,"scalar_offset":255}]"#
    ));
    assert!(!evidence_value(finding_257, "locations").contains("\"scalar_offset\":256"));
}

#[test]
fn compiled_path_preserves_valid_and_malformed_read_boundary_semantics() {
    assert_eq!(INSPECTION_READ_BOUNDARY, 65_536);
    let valid_report = inspect_fixture("valid_control_crosses_read_boundary").1;
    let valid = finding(&valid_report, BIDI_MECHANISM_ID);
    assert_eq!(valid.status(), FindingStatus::Present);
    assert_eq!(
        evidence_value(valid, "locations"),
        r#"[{"code_point":"U+202E","abbreviation":"RLO","byte_offset":65535,"scalar_offset":65535}]"#
    );
    assert_eq!(valid_report.artifact().byte_length(), 65_539);
    assert_eq!(
        valid_report.artifact().content_sha256().as_str(),
        "324277bc492569075d693fecfc01fc21f1c8d84beca5c79f3da98aaa5ed27131"
    );

    let malformed_report = inspect_fixture("malformed_utf8_crosses_read_boundary").1;
    assert_eq!(malformed_report.artifact().byte_length(), 131_074);
    assert_eq!(
        malformed_report.artifact().content_sha256().as_str(),
        "a8eefc7237d54ce856f771ccc02df988b09f1ede84eb3fc6e0fb64a5018a8be0"
    );
    for mechanism_id in [BIDI_MECHANISM_ID, DICP_MECHANISM_ID] {
        assert_invalid_without_property_evidence(
            finding(&malformed_report, mechanism_id),
            mechanism_id,
        );
    }
}

#[test]
fn actual_json_and_stderr_bytes_are_identical_across_eight_runs() {
    let fixture = named_fixture("all_control_identities");
    let artifact = TempArtifact::new(&fixture.input);
    let first = inspect(artifact.path());
    assert!(first.status.success(), "stderr: {}", stderr(&first));

    for run in 2..=8 {
        let next = inspect(artifact.path());
        assert!(next.status.success(), "run {run} stderr: {}", stderr(&next));
        assert_eq!(next.stdout, first.stdout, "run {run} stdout bytes");
        assert_eq!(next.stderr, first.stderr, "run {run} stderr bytes");
    }

    let report = Report::from_json(stdout(&first).trim_end()).expect("stdout is a report");
    assert_eq!(
        report
            .findings()
            .iter()
            .map(|finding| finding.mechanism().id())
            .collect::<Vec<_>>(),
        [
            BIDI_MECHANISM_ID,
            DICP_MECHANISM_ID,
            NFC_MECHANISM_ID,
            NFKC_MECHANISM_ID,
        ]
    );
    let bidi = finding(&report, BIDI_MECHANISM_ID);
    let ExpectedObservation::Valid(expected) = fixture.expected else {
        panic!("determinism fixture must be valid");
    };
    assert_eq!(bidi.evidence(), expected.report_evidence());
}

fn inspect_fixture(name: &str) -> (TempArtifact, Report) {
    let fixture = named_fixture(name);
    let artifact = TempArtifact::new(&fixture.input);
    let output = inspect(artifact.path());
    assert!(
        output.status.success(),
        "{name} stderr: {}",
        stderr(&output)
    );
    assert_eq!(stderr(&output), "", "{name} stderr");
    let report = Report::from_json(stdout(&output).trim_end()).expect("stdout is a report");
    (artifact, report)
}

fn named_fixture(name: &str) -> Fixture {
    fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == name)
        .unwrap_or_else(|| panic!("fixture {name} exists"))
}

fn finding<'a>(report: &'a Report, mechanism_id: &str) -> &'a Finding {
    report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == mechanism_id)
        .unwrap_or_else(|| panic!("finding {mechanism_id} exists"))
}

fn evidence_value<'a>(finding: &'a Finding, name: &str) -> &'a str {
    finding
        .evidence()
        .iter()
        .find(|evidence| evidence.name() == name)
        .unwrap_or_else(|| panic!("evidence {name} exists"))
        .value()
}

fn assert_invalid_without_property_evidence(finding: &Finding, context: &str) {
    assert_eq!(finding.status(), FindingStatus::Invalid, "{context} status");
    assert_eq!(
        finding.evidence(),
        [Evidence::new(
            "utf8_validation",
            "failed: the complete artifact is malformed or incomplete UTF-8",
        )],
        "{context} invalid evidence"
    );
    for absent_name in ["locations", "locations_truncated", "total_occurrence_count"] {
        assert!(
            finding
                .evidence()
                .iter()
                .all(|evidence| evidence.name() != absent_name),
            "{context} must discard {absent_name}"
        );
    }
}

fn inspect(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(path)
        .arg("--json")
        .output()
        .expect("scrub process can run")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr is UTF-8")
}
