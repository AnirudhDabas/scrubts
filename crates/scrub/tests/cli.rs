use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::{Evidence, FindingStatus, Report};

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const MULTI_BUFFER_SHA256: &str =
    "008ffc88d3c96a9f307524eb361e47c5222a887fc45fa0c1fb8d429c5c23b430";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempArtifact {
    directory: PathBuf,
    path: PathBuf,
}

impl TempArtifact {
    fn new(contents: &[u8]) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("scrub-cli-test-{}-{id}", std::process::id()));
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

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scrub"))
        .args(arguments)
        .output()
        .expect("scrub process can run")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr is UTF-8")
}

fn inspect_json(artifact: &TempArtifact) -> Report {
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    Report::from_json(stdout(&output).trim_end()).expect("stdout is a report")
}

#[test]
fn json_stdout_contains_only_the_canonical_report() {
    let artifact = TempArtifact::new(b"abc");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let json = stdout(&output)
        .strip_suffix('\n')
        .expect("JSON output has one trailing newline");
    assert!(!json.contains('\n'), "JSON report must occupy one line");

    let report = Report::from_json(json).expect("stdout is a report");
    assert_eq!(report.schema_version(), "0.1");
    assert_eq!(report.tool().name(), "scrub");
    assert_eq!(report.tool().version(), "0.1.0");
    assert_eq!(report.artifact().path(), artifact.path().to_string_lossy());
    assert_eq!(report.artifact().byte_length(), 3);
    assert_eq!(report.artifact().content_sha256().as_str(), ABC_SHA256);
    assert_eq!(report.findings().len(), 9);
    assert_eq!(
        report.findings()[0].mechanism().id(),
        "unicode.bidi_control"
    );
    assert_eq!(report.findings()[0].status(), FindingStatus::Absent);
    assert_eq!(
        report.findings()[0].evidence(),
        [
            Evidence::new("locations", "[]"),
            Evidence::new("locations_truncated", "false"),
            Evidence::new("total_occurrence_count", "0"),
        ]
    );
    let finding = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == "unicode.default_ignorable_code_point")
        .expect("DICP finding exists");
    assert_eq!(
        finding.mechanism().id(),
        "unicode.default_ignorable_code_point"
    );
    assert_eq!(finding.mechanism().version(), "17.0.0");
    assert_eq!(finding.status(), FindingStatus::Absent);
    assert_eq!(
        finding.evidence(),
        [
            Evidence::new("locations", "[]"),
            Evidence::new("locations_truncated", "false"),
            Evidence::new("total_occurrence_count", "0"),
        ]
    );
    assert_eq!(report.limitations().len(), 1);
    assert!(report.limitations()[0].contains("not evaluated"));
    assert!(report.limitations()[0].contains("Bidi_Control"));
    assert!(report.limitations()[0].contains("NFC-difference"));
    assert!(report.limitations()[0].contains("NFKC-difference"));
    assert!(!report.limitations()[0].contains("bidi-control"));
    assert!(report.assumptions().is_empty());
}

#[test]
fn json_reports_dicp_presence_without_changing_success_exit_semantics() {
    let artifact = TempArtifact::new("a\u{200b}b".as_bytes());
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let report = Report::from_json(stdout(&output).trim_end()).expect("stdout is a report");
    let finding = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == "unicode.default_ignorable_code_point")
        .expect("DICP finding exists");
    assert_eq!(finding.status(), FindingStatus::Present);
    assert_eq!(
        finding.evidence(),
        [
            Evidence::new(
                "locations",
                r#"[{"code_point":"U+200B","byte_offset":1,"scalar_offset":1}]"#,
            ),
            Evidence::new("locations_truncated", "false"),
            Evidence::new("total_occurrence_count", "1"),
        ]
    );
}

#[test]
fn empty_file_has_zero_length_and_empty_sha256() {
    let report = inspect_json(&TempArtifact::new(b""));

    assert_eq!(report.artifact().byte_length(), 0);
    assert_eq!(report.artifact().content_sha256().as_str(), EMPTY_SHA256);
}

#[test]
fn file_larger_than_read_buffer_is_fully_hashed() {
    let contents = vec![b'a'; 64 * 1024 + 1];
    let report = inspect_json(&TempArtifact::new(&contents));

    assert_eq!(report.artifact().byte_length(), 65_537);
    assert_eq!(
        report.artifact().content_sha256().as_str(),
        MULTI_BUFFER_SHA256
    );
}

#[test]
fn default_output_is_human_readable_and_neutral() {
    let artifact = TempArtifact::new("a\u{200b}b".as_bytes());
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(stdout(&output).contains("mechanism: Default_Ignorable_Code_Point (Unicode 17.0.0)"));
    assert!(stdout(&output).contains("mechanism: Bidi_Control (Unicode 17.0.0)"));
    assert!(stdout(&output).contains("status: present"));
    assert!(stdout(&output).contains("\"code_point\":\"U+200B\""));
    assert!(stdout(&output).contains("limitation:"));

    let normalized = stdout(&output).to_ascii_lowercase();
    for prohibited in [
        "suspicious",
        "malicious",
        "ai-generated",
        "watermark detected",
        "watermark removed",
        "clean",
        "safe",
    ] {
        assert!(
            !normalized.contains(prohibited),
            "prohibited wording: {prohibited}"
        );
    }
}

#[test]
fn human_output_reports_bidi_identity_without_rendering_the_control() {
    let artifact = TempArtifact::new("a\u{202e}b".as_bytes());
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(stdout(&output).contains("mechanism: Bidi_Control (Unicode 17.0.0)"));
    assert!(stdout(&output).contains("\"code_point\":\"U+202E\""));
    assert!(stdout(&output).contains("\"abbreviation\":\"RLO\""));
    assert!(stdout(&output).contains("\"byte_offset\":1"));
    assert!(stdout(&output).contains("\"scalar_offset\":1"));
    assert!(!stdout(&output).contains('\u{202e}'));

    let normalized = stdout(&output).to_ascii_lowercase();
    for prohibited in [
        "attack",
        "malicious",
        "dangerous",
        "suspicious",
        "trojan source",
    ] {
        assert!(
            !normalized.contains(prohibited),
            "prohibited wording: {prohibited}"
        );
    }
}

#[test]
fn usage_error_exits_two_and_keeps_stdout_empty() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("missing command"));
    assert!(stderr(&output).contains("Usage:"));
}

#[test]
fn unreadable_artifact_exits_one_and_keeps_stdout_empty() {
    let artifact = TempArtifact::new(b"abc");
    let missing = artifact.directory.join("missing.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(missing)
        .arg("--json")
        .output()
        .expect("scrub process can run");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("could not open"));
}

#[test]
fn directory_is_rejected_as_not_a_regular_file() {
    let artifact = TempArtifact::new(b"abc");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(&artifact.directory)
        .output()
        .expect("scrub process can run");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("not a regular file"));
}
