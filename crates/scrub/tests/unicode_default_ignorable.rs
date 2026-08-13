mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::{Evidence, FindingStatus, Report};
use sha2::{Digest, Sha256};
use support::unicode_default_ignorable::{
    ExpectedObservation, INSPECTION_READ_BOUNDARY, fixture_corpus, pinned_dicp_ranges,
};

const MECHANISM_ID: &str = "unicode.default_ignorable_code_point";
const UNICODE_VERSION: &str = "17.0.0";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn frozen_membership_oracle_is_available_to_real_path_tests() {
    let ranges = pinned_dicp_ranges();
    assert_eq!(ranges.len(), 27);
    assert_eq!(
        ranges
            .iter()
            .copied()
            .map(|range| range.code_point_count())
            .sum::<u32>(),
        4_174
    );
    assert!(ranges.iter().copied().any(|range| range.contains(0x200B)));
    assert!(!ranges.iter().copied().any(|range| range.contains(0x0301)));
}

struct TempArtifact {
    directory: PathBuf,
    path: PathBuf,
}

impl TempArtifact {
    fn new(contents: &[u8]) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("scrub-unicode-test-{}-{id}", std::process::id()));
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
fn real_inspection_path_matches_the_complete_fixture_corpus() {
    for fixture in fixture_corpus() {
        let artifact = TempArtifact::new(&fixture.input);
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
        assert_eq!(report.findings().len(), 9, "{} finding count", fixture.name);
        let finding = report
            .findings()
            .iter()
            .find(|finding| finding.mechanism().id() == MECHANISM_ID)
            .expect("DICP finding exists");
        assert_eq!(
            finding.mechanism().id(),
            MECHANISM_ID,
            "{} id",
            fixture.name
        );
        assert_eq!(
            finding.mechanism().version(),
            UNICODE_VERSION,
            "{} version",
            fixture.name
        );
        assert_eq!(
            finding.status(),
            fixture.expected.status(),
            "{} status",
            fixture.name
        );

        match &fixture.expected {
            ExpectedObservation::Valid(expected) => {
                assert_eq!(
                    finding.evidence(),
                    expected.report_evidence(),
                    "{} evidence",
                    fixture.name
                );
            }
            ExpectedObservation::InvalidUtf8 => {
                assert_eq!(
                    finding.evidence(),
                    [Evidence::new(
                        "utf8_validation",
                        "failed: the complete artifact is malformed or incomplete UTF-8",
                    )],
                    "{} invalid evidence",
                    fixture.name
                );
                assert!(
                    finding
                        .limitations()
                        .iter()
                        .any(|value| value.contains("complete artifact is not valid UTF-8")),
                    "{} invalid limitation",
                    fixture.name
                );
                assert_ne!(
                    finding.status(),
                    FindingStatus::Absent,
                    "{} status",
                    fixture.name
                );
            }
        }
    }
}

#[test]
fn malformed_utf8_carried_across_read_boundary_invalidates_prefix_evidence() {
    let mut input = "\u{200b}".as_bytes().to_vec();
    input.resize(INSPECTION_READ_BOUNDARY - 1, b'a');
    input.push(0xe0);
    input.push(0x80);
    input.resize(INSPECTION_READ_BOUNDARY * 3, b'z');
    assert_eq!(input.len(), 196_608);

    let expected_digest: [u8; 32] = Sha256::digest(&input).into();
    let artifact = TempArtifact::new(&input);
    let output = inspect(artifact.path());

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let json = stdout(&output)
        .strip_suffix('\n')
        .expect("JSON output has one trailing newline");
    assert!(!json.contains('\n'), "JSON report must occupy one line");

    let report = Report::from_json(json).expect("stdout is a report");
    assert_eq!(report.artifact().byte_length(), 196_608);
    assert_eq!(
        report.artifact().content_sha256().as_str(),
        scrub_report::Sha256Digest::from_bytes(expected_digest).as_str()
    );
    assert_eq!(report.findings().len(), 9);
    let finding = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == MECHANISM_ID)
        .expect("DICP finding exists");
    assert_eq!(finding.mechanism().id(), MECHANISM_ID);
    assert_eq!(finding.mechanism().version(), UNICODE_VERSION);
    assert_eq!(finding.status(), FindingStatus::Invalid);
    assert_eq!(
        finding.evidence(),
        [Evidence::new(
            "utf8_validation",
            "failed: the complete artifact is malformed or incomplete UTF-8",
        )]
    );
    for absent_name in ["total_occurrence_count", "locations_truncated", "locations"] {
        assert!(
            finding
                .evidence()
                .iter()
                .all(|evidence| evidence.name() != absent_name),
            "{absent_name} evidence must be discarded"
        );
    }

    assert_eq!(
        fs::read(artifact.path()).expect("artifact can be read after inspection"),
        input
    );
}

#[test]
fn real_inspection_path_does_not_modify_the_artifact() {
    let fixture = fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == "dicp_spans_64_kib_read_boundary")
        .expect("boundary fixture exists");
    let artifact = TempArtifact::new(&fixture.input);
    let before = fs::read(artifact.path()).expect("artifact can be read before inspection");
    let before_digest: [u8; 32] = Sha256::digest(&before).into();

    let output = inspect(artifact.path());
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let after = fs::read(artifact.path()).expect("artifact can be read after inspection");
    let after_digest: [u8; 32] = Sha256::digest(&after).into();
    assert_eq!(after, before);
    assert_eq!(after_digest, before_digest);
}

#[test]
fn report_output_is_deterministic_for_the_same_artifact() {
    let artifact = TempArtifact::new("a\u{200b}b".as_bytes());
    let first = inspect(artifact.path());
    let second = inspect(artifact.path());

    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
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
