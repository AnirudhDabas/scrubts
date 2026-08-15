use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use scrub::inspect_reader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SUPPORTED_PLATFORMS: [&str; 3] = ["windows", "linux", "macos"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    semantic_digest_generation_command: Vec<String>,
    fixtures: Vec<ManifestFixture>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFixture {
    fixture_id: String,
    path: String,
    byte_sha256: String,
    expected_capability: String,
}

#[derive(Serialize)]
struct PlatformResult {
    schema_version: &'static str,
    project_revision: String,
    platform: String,
    fixtures: Vec<FixtureResult>,
}

#[derive(Serialize)]
struct FixtureResult {
    fixture_id: String,
    input_sha256: String,
    expected_capability: String,
    semantic_report_sha256: String,
}

struct Arguments {
    manifest: PathBuf,
    platform: String,
    project_revision: String,
    output: PathBuf,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut arguments = env::args().skip(1);
    let mut manifest = None;
    let mut platform = None;
    let mut project_revision = None;
    let mut output = None;
    while let Some(option) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--platform" => platform = Some(value),
            "--project-revision" => project_revision = Some(value),
            "--output" => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown option: {option}")),
        }
    }
    Ok(Arguments {
        manifest: manifest.ok_or("missing --manifest")?,
        platform: platform.ok_or("missing --platform")?,
        project_revision: project_revision.ok_or("missing --project-revision")?,
        output: output.ok_or("missing --output")?,
    })
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn validate_platform(asserted: &str, actual: &str) -> Result<(), String> {
    if !SUPPORTED_PLATFORMS.contains(&asserted) {
        return Err(format!(
            "unsupported --platform {asserted:?}; expected one of windows, linux, macos"
        ));
    }
    if !SUPPORTED_PLATFORMS.contains(&actual) {
        return Err(format!("unsupported compilation target OS {actual:?}"));
    }
    if asserted != actual {
        return Err(format!(
            "--platform {asserted:?} does not match the binary target OS {actual:?}"
        ));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let actual_platform = env::consts::OS;
    validate_platform(&arguments.platform, actual_platform)?;
    if arguments.project_revision.len() != 40
        || !arguments
            .project_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("--project-revision must be a lowercase 40-hex Git identity".to_owned());
    }
    let root = repository_root();
    let manifest_path = root.join(&arguments.manifest);
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| format!("invalid determinism manifest: {error}"))?;
    if manifest.schema_version != "0.1" || manifest.semantic_digest_generation_command.is_empty() {
        return Err("unsupported or incomplete determinism manifest".to_owned());
    }

    let mut fixture_results = Vec::with_capacity(manifest.fixtures.len());
    for fixture in manifest.fixtures {
        let path = root.join(&fixture.path);
        let bytes = fs::read(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let input_sha256 = sha256(&bytes);
        if input_sha256 != fixture.byte_sha256 {
            return Err(format!(
                "fixture byte identity mismatch for {}: expected {}, got {}",
                fixture.fixture_id, fixture.byte_sha256, input_sha256
            ));
        }
        let report = inspect_reader(Path::new(&fixture.path), &mut Cursor::new(&bytes))
            .map_err(|error| format!("inspection failed for {}: {error}", fixture.fixture_id))?;
        let semantic = report
            .canonical_report_bytes()
            .map_err(|error| format!("canonical report failed: {error}"))?;
        fixture_results.push(FixtureResult {
            fixture_id: fixture.fixture_id,
            input_sha256,
            expected_capability: fixture.expected_capability,
            semantic_report_sha256: sha256(&semantic),
        });
    }
    fixture_results.sort_by(|left, right| left.fixture_id.cmp(&right.fixture_id));
    let result = PlatformResult {
        schema_version: "0.1",
        project_revision: arguments.project_revision,
        platform: actual_platform.to_owned(),
        fixtures: fixture_results,
    };
    if let Some(parent) = arguments.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let mut encoded = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("could not serialize platform result: {error}"))?;
    encoded.push('\n');
    fs::write(&arguments.output, encoded)
        .map_err(|error| format!("could not write {}: {error}", arguments.output.display()))?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("determinism generation failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_target_platform_is_accepted() {
        assert!(SUPPORTED_PLATFORMS.contains(&env::consts::OS));
        assert_eq!(validate_platform(env::consts::OS, env::consts::OS), Ok(()));
    }

    #[test]
    fn every_other_supported_platform_is_rejected() {
        for platform in SUPPORTED_PLATFORMS {
            if platform != env::consts::OS {
                let error = validate_platform(platform, env::consts::OS)
                    .expect_err("another supported platform cannot impersonate this target");
                assert!(error.contains("does not match the binary target OS"));
            }
        }
    }

    #[test]
    fn arbitrary_and_alias_platforms_are_rejected() {
        for platform in ["local-windows", "win32", "darwin", "arbitrary"] {
            let error = validate_platform(platform, env::consts::OS)
                .expect_err("noncanonical platform must be rejected");
            assert!(error.contains("unsupported --platform"));
        }
    }
}
