use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use scrub_report::{human_safe, is_forbidden_human_control};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn hostile_human_strings_are_visibly_escaped() {
    let hostile = concat!(
        "esc=\u{1b}[31m",
        " osc=\u{1b}]0;title\u{7}",
        " link=\u{1b}]8;;https://example.invalid\u{7}label\u{1b}]8;;\u{7}",
        " layout=\r\n\t\u{8}",
        " c0=\u{0}\u{1f}\u{7f}",
        " c1=\u{80}\u{9b}",
        " bidi=\u{61c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}",
        " isolates=\u{2066}\u{2067}\u{2068}\u{2069}",
    );
    let escaped = human_safe(hostile);

    assert!(
        escaped
            .chars()
            .all(|character| !is_forbidden_human_control(character))
    );
    for visible in [
        "\\u{1b}",
        "\\u{7}",
        "\\u{d}",
        "\\u{a}",
        "\\u{9}",
        "\\u{8}",
        "\\u{0}",
        "\\u{1f}",
        "\\u{7f}",
        "\\u{80}",
        "\\u{9b}",
        "\\u{61c}",
        "\\u{200e}",
        "\\u{200f}",
        "\\u{202e}",
        "\\u{2066}",
        "\\u{2069}",
    ] {
        assert!(
            escaped.contains(visible),
            "missing visible escape {visible}"
        );
    }

    let long = format!("{}{}", "x".repeat(10_000), hostile);
    let long_escaped = human_safe(&long);
    assert!(long_escaped.starts_with(&"x".repeat(10_000)));
    assert!(
        long_escaped
            .chars()
            .all(|character| !is_forbidden_human_control(character))
    );
}

#[test]
fn hostile_filename_cannot_change_human_output_layout() {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "scrub-terminal-output-safety-{}-{id}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("temporary directory can be created");
    let path = directory.join("status\u{202e}txt.artifact");
    fs::write(&path, b"ordinary text").expect("temporary artifact can be written");

    let output = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(&path)
        .output()
        .expect("scrub process can run");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = std::str::from_utf8(&output.stdout).expect("human output is UTF-8");
    assert!(stdout.contains("status\\u{202e}txt.artifact"));
    assert!(!stdout.contains('\u{202e}'));
    assert!(
        stdout
            .chars()
            .all(|character| character == '\n' || !is_forbidden_human_control(character))
    );

    fs::remove_file(&path).expect("temporary artifact can be removed");
    fs::remove_dir(&directory).expect("temporary directory can be removed");
}
