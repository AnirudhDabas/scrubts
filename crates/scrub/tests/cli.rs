use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::Report;

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
    assert!(report.findings().is_empty());
    assert_eq!(
        report.limitations(),
        ["Milestone 1 does not implement artifact scanners."]
    );
    assert!(report.assumptions().is_empty());
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
fn default_output_is_human_readable_without_an_absence_claim() {
    let artifact = TempArtifact::new(b"abc");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .output()
        .expect("scrub process can run");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(stdout(&output).contains("findings: 0 (no scanners run)"));
    assert!(
        stdout(&output).contains("limitation: Milestone 1 does not implement artifact scanners.")
    );
    assert!(!stdout(&output).to_ascii_lowercase().contains("absent"));
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
