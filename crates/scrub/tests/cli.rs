use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::{Evidence, FindingStatus, InferenceId, Report, VerifierAvailability};

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

#[test]
fn root_help_is_successful_and_lists_only_the_existing_surface() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    assert_eq!(stderr(&help), "");
    assert_eq!(
        stdout(&help),
        concat!(
            "Usage:\n",
            "  scrub inspect <path>\n",
            "  scrub inspect <path> --explain\n",
            "  scrub inspect <path> --json\n",
            "\n",
            "Command:\n",
            "  inspect  Inspect one local artifact without network access\n",
            "\n",
            "Global options:\n",
            "  --help     Show this help\n",
            "  --version  Show package version\n",
        )
    );
}

#[test]
fn inspect_help_is_successful_and_describes_real_options() {
    let help = run(&["inspect", "--help"]);
    assert!(help.status.success());
    assert_eq!(stderr(&help), "");
    assert_eq!(
        stdout(&help),
        concat!(
            "Usage:\n",
            "  scrub inspect <path>\n",
            "  scrub inspect <path> --explain\n",
            "  scrub inspect <path> --json\n",
            "\n",
            "Options:\n",
            "  --explain  Show complete evidence and authority chain in human output\n",
            "  --json     Emit the report as JSON on stdout\n",
            "  --help     Show this help\n",
        )
    );
}

#[test]
fn version_is_successful_and_comes_from_the_package_version() {
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert_eq!(stderr(&version), "");
    assert_eq!(
        stdout(&version),
        format!("scrub {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_with_extra_arguments_remains_a_usage_error() {
    let output = run(&["inspect", "--help", "artifact.txt"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("unknown option: --help"));
    assert!(stderr(&output).contains("Usage:"));
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
    Report::from_json(stdout(&output).trim_end())
        .expect("stdout is an untrusted report")
        .into_report()
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

    let report = Report::from_json(json)
        .expect("stdout is an untrusted report")
        .into_report();
    assert_eq!(report.schema_version(), "0.2");
    assert_eq!(report.tool().name(), "scrub");
    assert_eq!(report.tool().version(), "0.1.0");
    assert_eq!(report.artifact().path(), "artifact.txt");
    assert_eq!(report.artifact().byte_length(), 3);
    assert_eq!(report.artifact().content_sha256().as_str(), ABC_SHA256);
    assert_eq!(report.findings().len(), 10);
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
    assert!(report.limitations()[0].contains("listed Unicode and C2PA"));
    assert!(report.limitations()[0].contains("provider-detector availability"));
    assert!(report.limitations()[0].contains("public-reference statistical detectors"));
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
    let report = Report::from_json(stdout(&output).trim_end())
        .expect("stdout is an untrusted report")
        .into_report();
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
    assert!(stdout(&output).contains("PRESENT         Default_Ignorable_Code_Point"));
    assert!(stdout(&output).contains("U+200B at byte offset 1, scalar offset 1"));
    assert!(stdout(&output).contains("UNKNOWN         embedded text watermark"));
    assert!(stdout(&output).contains("unavailable in checked authority snapshot"));
    assert!(stdout(&output).contains("related family; not deployment parity"));
    assert!(
        stdout(&output)
            .contains("does not support  Claude watermark presence/absence or provider parity")
    );
    assert!(stdout(&output).contains("UNKNOWN != ABSENT / CLEAN"));
    assert!(stdout(&output).contains("Use --explain"));

    let normalized = stdout(&output).to_ascii_lowercase();
    for prohibited in [
        "suspicious",
        "malicious",
        "ai-generated",
        "watermark detected",
        "watermark removed",
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
    assert!(stdout(&output).contains("PRESENT         Bidi_Control"));
    assert!(stdout(&output).contains("U+202E (RLO) at byte offset 1, scalar offset 1"));
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

#[test]
fn explain_projects_the_typed_provider_authority_boundary() {
    let artifact = TempArtifact::new(b"ordinary text");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--explain")
        .output()
        .expect("scrub process can run");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let explanation = stdout(&output);
    assert!(explanation.contains("anthropic.embedded_text_watermark"));
    assert!(explanation.contains("status            UNKNOWN"));
    assert!(explanation.contains("verifier          anthropic.provider_detector unpublished"));
    assert!(explanation.contains("availability      unavailable in checked authority snapshot"));
    assert!(explanation.contains("anthropic-claude-text-watermark"));
    assert!(explanation.contains("related reference reference.synthid_text"));
    assert!(explanation.contains("related family; not deployment parity"));
    assert!(explanation.contains("provider_detector_unavailable"));
    assert!(explanation.contains("claude_watermark_absent"));
    assert!(explanation.contains("does not support  artifact_clean"));
    assert!(!explanation.contains("does not supportartifact_clean"));
    assert!(!explanation.contains("status            CLEAN"));
    assert!(!explanation.contains("status            HUMAN"));
}

#[test]
fn human_and_explain_outputs_are_byte_deterministic() {
    let artifact = TempArtifact::new("text\u{200b}".as_bytes());
    for options in [vec![], vec!["--explain"]] {
        let mut arguments = vec!["inspect", artifact.path().to_str().expect("UTF-8 path")];
        arguments.extend(options);
        let first = run(&arguments);
        assert!(first.status.success(), "stderr: {}", stderr(&first));
        for repetition in 0..3 {
            let repeated = run(&arguments);
            assert!(
                repeated.status.success(),
                "repetition {repetition} stderr: {}",
                stderr(&repeated)
            );
            assert_eq!(repeated.stdout, first.stdout, "repetition {repetition}");
            assert_eq!(repeated.stderr, first.stderr, "repetition {repetition}");
        }
    }
}

#[test]
fn json_explain_is_the_same_structured_report_not_generated_prose() {
    let artifact = TempArtifact::new(b"ordinary text");
    let plain = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .output()
        .expect("scrub process can run");
    let explained = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .arg("--explain")
        .output()
        .expect("scrub process can run");
    assert!(plain.status.success());
    assert!(explained.status.success());
    assert_eq!(plain.stdout, explained.stdout);
    assert!(explained.stderr.is_empty());
    let report = Report::from_json(stdout(&explained).trim_end())
        .expect("structured untrusted report")
        .into_report();
    let provider = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == "anthropic.embedded_text_watermark")
        .expect("provider finding exists");
    assert_eq!(provider.status(), FindingStatus::Unknown);
    assert_eq!(
        provider.trace().verifier().availability(),
        VerifierAvailability::Unavailable
    );
    assert!(
        provider
            .trace()
            .does_not_support()
            .contains(&InferenceId::ClaudeProviderParity)
    );
}

#[test]
fn json_uses_only_a_display_name_and_has_no_terminal_controls() {
    let artifact = TempArtifact::new(b"ordinary text");
    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(artifact.path())
        .arg("--json")
        .output()
        .expect("scrub process can run");
    assert!(output.status.success());
    assert!(!stdout(&output).contains(&artifact.directory.to_string_lossy().to_string()));
    assert!(!output.stdout.contains(&0x1b));
    assert!(!stdout(&output).contains("]8;"));
}

#[test]
fn bidi_filename_is_escaped_for_humans_but_json_remains_machine_data() {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("scrub-cli-bidi-name-{}-{id}", std::process::id()));
    fs::create_dir(&directory).expect("temporary directory can be created");
    let path = directory.join("safe\u{202e}txt.artifact");
    fs::write(&path, b"ordinary text").expect("temporary artifact can be written");

    let human = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(&path)
        .output()
        .expect("scrub process can run");
    assert!(human.status.success(), "stderr: {}", stderr(&human));
    assert!(stdout(&human).contains("safe\\u{202e}txt.artifact"));
    assert!(!stdout(&human).contains('\u{202e}'));

    let json = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(&path)
        .arg("--json")
        .output()
        .expect("scrub process can run");
    assert!(json.status.success(), "stderr: {}", stderr(&json));
    let report = Report::from_json(stdout(&json).trim_end())
        .expect("machine JSON remains standards-valid")
        .into_report();
    assert_eq!(report.artifact().path(), "safe\u{202e}txt.artifact");

    fs::remove_file(&path).expect("temporary artifact can be removed");
    fs::remove_dir(&directory).expect("temporary directory can be removed");
}

#[test]
fn malformed_text_is_invalid_for_unicode_but_claude_remains_unknown() {
    let artifact = TempArtifact::new(&[0xff]);
    let report = inspect_json(&artifact);
    let unicode = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == "unicode.bidi_control")
        .expect("Unicode finding exists");
    let provider = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == "anthropic.embedded_text_watermark")
        .expect("provider finding exists");
    assert_eq!(unicode.status(), FindingStatus::Invalid);
    assert_eq!(provider.status(), FindingStatus::Unknown);
    assert!(
        provider
            .trace()
            .supports()
            .contains(&InferenceId::ProviderDetectorUnavailable)
    );
    assert!(
        provider
            .trace()
            .does_not_support()
            .contains(&InferenceId::ClaudeWatermarkAbsent)
    );
}
