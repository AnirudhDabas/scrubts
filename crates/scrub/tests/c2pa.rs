use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use c2pa::{Context, Reader, Settings};
use scrub_report::{Finding, FindingStatus, Report, is_forbidden_human_control};
use sha2::{Digest, Sha256};

const FIXTURES: &str = "tests/fixtures/c2pa";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("scrub-c2pa-test-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("temporary directory can be created");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES)
        .join(relative)
}

fn run(path: &Path, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrub"));
    command.arg("inspect").arg(path);
    if json {
        command.arg("--json");
    }
    command.output().expect("scrub process can run")
}

fn report(path: &Path) -> (Report, String) {
    let output = run(path, true);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let raw = String::from_utf8(output.stdout).expect("JSON stdout is UTF-8");
    let report = Report::from_json(raw.trim_end())
        .expect("stdout is an untrusted report")
        .into_report();
    (report, raw)
}

fn finding<'a>(report: &'a Report, id: &str) -> &'a Finding {
    report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == id)
        .expect("finding exists")
}

fn evidence<'a>(finding: &'a Finding, name: &str) -> &'a str {
    finding
        .evidence()
        .iter()
        .find(|evidence| evidence.name() == name)
        .expect("evidence exists")
        .value()
}

fn digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn png_chunk(bytes: &[u8], kind: &[u8; 4]) -> Option<(std::ops::Range<usize>, usize)> {
    let mut offset = 8_usize;
    while offset.checked_add(12)? <= bytes.len() {
        let length = usize::try_from(u32::from_be_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
        .ok()?;
        let data_start = offset.checked_add(8)?;
        let data_end = data_start.checked_add(length)?;
        let crc_offset = data_end;
        if crc_offset.checked_add(4)? > bytes.len() {
            return None;
        }
        if bytes.get(offset + 4..offset + 8)? == kind {
            return Some((data_start..data_end, crc_offset));
        }
        offset = crc_offset + 4;
    }
    None
}

fn png_crc(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low_bit_mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & low_bit_mask);
        }
    }
    !crc
}

fn mutate_png_chunk(bytes: &mut [u8], kind: &[u8; 4], data_offset: usize) {
    let (data, crc_offset) = png_chunk(bytes, kind).expect("requested PNG chunk exists");
    let mutation = data.start + data_offset.min(data.len().saturating_sub(1));
    bytes[mutation] ^= 1;
    let type_start = data.start - 4;
    let crc = png_crc(&bytes[type_start..data.end]);
    bytes[crc_offset..crc_offset + 4].copy_from_slice(&crc.to_be_bytes());
}

fn replace_equal_length(bytes: &mut [u8], old: &[u8], new: &[u8]) {
    assert_eq!(old.len(), new.len());
    let start = bytes
        .windows(old.len())
        .position(|window| window == old)
        .expect("expected source bytes exist");
    bytes[start..start + old.len()].copy_from_slice(new);
}

fn byte_offsets(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == needle).then_some(offset))
        .collect()
}

fn phase_one_reader(bytes: Vec<u8>) -> Reader {
    let settings = Settings::new()
        .with_json(
            r#"{
                "core": {"allowed_network_hosts": [], "decode_identity_assertions": false},
                "verify": {
                    "verify_after_reading": false,
                    "verify_trust": false,
                    "verify_timestamp_trust": false,
                    "ocsp_fetch": false,
                    "remote_manifest_fetch": false
                }
            }"#,
        )
        .expect("fixed test settings are valid");
    Reader::from_context(
        Context::new()
            .with_settings(settings)
            .expect("fixed test context is valid"),
    )
    .with_stream("image/jpeg", Cursor::new(bytes))
    .expect("Phase 1 reader can structurally parse the derivative")
}

#[test]
fn frozen_fixture_identities_are_independently_recomputed() {
    for (relative, length, sha256) in [
        (
            "public-testfiles/adobe-20220124-CA.jpg",
            178_709,
            "cafc48c53e651f7ba4622d1f72783827074211e42b9634cc863ec3be3c7651b3",
        ),
        (
            "public-testfiles/adobe-20220124-E-dat-CA.jpg",
            178_709,
            "dae9d121060cec4b6f27ee8acda85ad461cf75f2261d90b463319b787342d7f9",
        ),
        (
            "public-testfiles/adobe-20220124-E-sig-CA.jpg",
            178_709,
            "0d4c2774f1b7e94b9613bb952b0a76b6a178d22ac6d206d257d2af1376cbbff2",
        ),
        (
            "public-testfiles/adobe-20220124-E-clm-CAICAI.jpg",
            656_258,
            "b3ff3f00c66602280977d3e4d962a836d33ab83953d0009f7c1e6490d0065feb",
        ),
        (
            "generated/signed.png",
            312_595,
            "276e64f0ba1f0ed3cd153f5fb166fb1864fadd03fd6d3cd5427cc77fc935fdb0",
        ),
        (
            "generated/signed.svg",
            44_421,
            "296c5e254427620ff3aef3176adf64e0311986775668013c33526c8ad1fc6fde",
        ),
        (
            "c2pa-rs/ocsp.jpg",
            285_562,
            "49a6b089bf3fe610960ef91b2beb81c86b62e7d531c97f33fc029841d864b2cb",
        ),
        (
            "c2pa-rs/ocsp_with_assertion.jpg",
            599_791,
            "210fb95c6a766d3cd89ef0583898ec7248fe60f0ed651af216fb270cd9cbe17a",
        ),
    ] {
        let bytes = fs::read(fixture(relative)).expect("fixture can be read");
        assert_eq!(bytes.len(), length, "{relative} length");
        assert_eq!(digest(&bytes), sha256, "{relative} SHA-256");
    }
}

#[test]
fn official_known_good_jpeg_has_independent_layered_results() {
    let path = fixture("public-testfiles/adobe-20220124-CA.jpg");
    let original = fs::read(&path).expect("fixture can be read");
    let (report, raw) = report(&path);

    assert_eq!(report.artifact().byte_length(), 178_709);
    assert_eq!(
        report.artifact().content_sha256().as_str(),
        digest(&original)
    );
    assert_eq!(
        finding(&report, "unicode.bidi_control").status(),
        FindingStatus::Invalid
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_validation").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.hard_binding").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.credential_trust").status(),
        FindingStatus::Unknown
    );
    assert_eq!(
        evidence(finding(&report, "c2pa.hard_binding"), "validation_code"),
        "assertion.dataHash.match"
    );
    assert_eq!(fs::read(&path).expect("fixture remains readable"), original);

    let ids = [
        "unicode.bidi_control",
        "unicode.default_ignorable_code_point",
        "unicode.normalization.nfc_difference",
        "unicode.normalization.nfkc_difference",
        "c2pa.text_manifest_wrapper",
        "c2pa.manifest_store",
        "c2pa.manifest_validation",
        "c2pa.hard_binding",
        "c2pa.credential_trust",
    ];
    let mut previous = 0;
    for id in ids {
        let position = raw
            .find(&format!("\"id\":\"{id}\""))
            .expect("raw JSON contains mechanism ID");
        assert!(position >= previous, "raw JSON order for {id}");
        previous = position;
    }
}

#[test]
fn official_tamper_vectors_do_not_collapse_signature_binding_and_structure() {
    let cases = [
        (
            "public-testfiles/adobe-20220124-E-dat-CA.jpg",
            FindingStatus::Invalid,
            FindingStatus::Invalid,
            Some("assertion.dataHash.mismatch"),
        ),
        (
            "public-testfiles/adobe-20220124-E-sig-CA.jpg",
            FindingStatus::Unknown,
            FindingStatus::Unknown,
            None,
        ),
        (
            "public-testfiles/adobe-20220124-E-clm-CAICAI.jpg",
            FindingStatus::Unknown,
            FindingStatus::Unknown,
            None,
        ),
    ];
    for (relative, validation, binding, binding_code) in cases {
        let (report, _) = report(&fixture(relative));
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::Present,
            "{relative} store"
        );
        assert_eq!(
            finding(&report, "c2pa.manifest_validation").status(),
            validation,
            "{relative} validation"
        );
        assert_eq!(
            finding(&report, "c2pa.hard_binding").status(),
            binding,
            "{relative} binding"
        );
        if let Some(binding_code) = binding_code {
            assert_eq!(
                evidence(finding(&report, "c2pa.hard_binding"), "validation_code"),
                binding_code,
                "{relative} binding code"
            );
        } else {
            assert_eq!(
                evidence(
                    finding(&report, "c2pa.manifest_validation"),
                    "validation_time_basis"
                ),
                "not_reproducible",
                "{relative} validation basis"
            );
            assert_eq!(
                evidence(
                    finding(&report, "c2pa.hard_binding"),
                    "validation_time_basis"
                ),
                "not_reproducible",
                "{relative} binding basis"
            );
        }
    }
}

#[test]
fn selected_c2pa_attacks_rendering_payload_has_an_explicit_scrub_contract() {
    // Exact 10-byte line from attacks/rendering.attack at pinned c2pa-attacks
    // commit 4f750daa888d2ff93a1659fc016be584dc43ae5c.
    let hostile_title = b"Back\x08Space";
    let source = fixture("generated/signed.png");
    let original = fs::read(&source).expect("generated source fixture can be read");
    let occurrences = original
        .windows(b"signed-png".len())
        .filter(|window| *window == b"signed-png")
        .count();
    assert_eq!(occurrences, 1, "generation replacement must be unambiguous");

    let mut derivative = original.clone();
    replace_equal_length(&mut derivative, b"signed-png", hostile_title);
    assert_eq!(derivative.len(), original.len());
    assert_ne!(digest(&derivative), digest(&original));

    let temp = TempDirectory::new();
    let path = temp.join("rendering-backspace-title.png");
    fs::write(&path, &derivative).expect("adversarial derivative can be written");
    let (report, _) = report(&path);
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_validation").status(),
        FindingStatus::Unknown,
        "untimestamped validation must not be upgraded"
    );
    assert_eq!(
        finding(&report, "c2pa.hard_binding").status(),
        FindingStatus::Unknown
    );
    assert_eq!(
        finding(&report, "c2pa.credential_trust").status(),
        FindingStatus::NotApplicable
    );

    let human = run(&path, false);
    assert!(
        human.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&human.stderr)
    );
    let human = String::from_utf8(human.stdout).expect("human output is UTF-8");
    assert!(!human.contains('\u{8}'));
    assert!(!human.contains("BackSpace"));
    assert!(
        !human
            .chars()
            .any(|scalar| scalar != '\n' && is_forbidden_human_control(scalar))
    );
    assert_eq!(
        fs::read(&source).expect("source remains readable"),
        original
    );
}

#[test]
fn png_and_svg_stream_handlers_validate_stores_and_hard_bindings() {
    for relative in ["generated/signed.png", "generated/signed.svg"] {
        let (report, _) = report(&fixture(relative));
        assert_eq!(
            finding(&report, "c2pa.text_manifest_wrapper").status(),
            FindingStatus::NotApplicable,
            "{relative} wrapper"
        );
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::Present,
            "{relative} store"
        );
        assert_eq!(
            finding(&report, "c2pa.manifest_validation").status(),
            FindingStatus::Unknown,
            "{relative} untimestamped integrity time basis"
        );
        assert_eq!(
            finding(&report, "c2pa.hard_binding").status(),
            FindingStatus::Unknown,
            "{relative} binding has no reproducible validation basis"
        );
        assert_eq!(
            evidence(
                finding(&report, "c2pa.manifest_validation"),
                "validation_time_basis"
            ),
            "not_reproducible"
        );
        assert_eq!(
            evidence(
                finding(&report, "c2pa.hard_binding"),
                "validation_time_basis"
            ),
            "not_reproducible"
        );
    }
}

#[test]
fn non_reproducible_clock_sensitive_artifacts_are_not_validated() {
    for relative in [
        "generated/signed.png",
        "generated/signed.svg",
        "c2pa-rs/ocsp_with_assertion.jpg",
    ] {
        let (report, _) = report(&fixture(relative));
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::Present,
            "{relative} structural store presence"
        );
        for id in ["c2pa.manifest_validation", "c2pa.hard_binding"] {
            let finding = finding(&report, id);
            assert_eq!(finding.status(), FindingStatus::Unknown, "{relative} {id}");
            assert_eq!(
                evidence(finding, "validation_time_basis"),
                "not_reproducible",
                "{relative} {id}"
            );
        }
    }

    let (timestamped, _) = report(&fixture("c2pa-rs/ocsp.jpg"));
    assert_eq!(
        finding(&timestamped, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&timestamped, "c2pa.manifest_validation").status(),
        FindingStatus::Present
    );
    assert_eq!(
        evidence(
            finding(&timestamped, "c2pa.manifest_validation"),
            "validation_time_basis"
        ),
        "validated_timestamp"
    );
    assert_eq!(
        finding(&timestamped, "c2pa.hard_binding").status(),
        FindingStatus::Present
    );
}

#[test]
fn omitted_untimestamped_referenced_claim_cannot_authorize_phase_two() {
    const ACTIVE: &str = "adobe:urn:uuid:8208d145-e90c-43f6-b9a4-58bd7ba87f80";
    const INTERMEDIATE: &str = "adobe:urn:uuid:17d82661-2e6e-48f1-922c-2e853cbeadfd";
    const OMITTED: &str = "urn:uuid:b82c36da-6dc9-418d-bd2c-9587009f797f";

    let mut derivative =
        fs::read(fixture("c2pa-rs/ocsp_with_assertion.jpg")).expect("fixture can be read");
    let hash_labels = byte_offsets(&derivative, b"c2pa.hash.data");
    assert_eq!(hash_labels, [3394, 3928, 417259, 418338, 541765, 542741]);
    for offset in &hash_labels[..2] {
        derivative[*offset..*offset + 14].copy_from_slice(b"c2pa.actions_1");
    }
    assert_eq!(derivative.len(), 599_791);
    assert_eq!(
        digest(&derivative),
        "0c9948826452dc34f43ee252e04834fc7b903c5c2b25664890829881c81e3e5f"
    );

    let reader = phase_one_reader(derivative.clone());
    assert_eq!(reader.active_label(), Some(ACTIVE));
    assert_eq!(reader.manifests().len(), 2);
    assert!(
        reader
            .active_manifest()
            .and_then(|manifest| manifest.signature_info())
            .is_some_and(|signature| signature.time.is_some())
    );
    assert_eq!(
        reader
            .get_manifest(INTERMEDIATE)
            .and_then(|manifest| manifest.ingredients().first())
            .and_then(|ingredient| ingredient.active_manifest()),
        Some(OMITTED)
    );
    assert!(reader.get_manifest(OMITTED).is_none());
    assert!(reader.validation_status().is_some_and(|statuses| {
        statuses.iter().any(|status| {
            status.code() == "assertion.required.missing"
                && status.url()
                    == Some("self#jumbf=/c2pa/urn:uuid:b82c36da-6dc9-418d-bd2c-9587009f797f")
        })
    }));

    let temp = TempDirectory::new();
    let path = temp.join("omitted-untimestamped-ingredient.jpg");
    fs::write(&path, derivative).expect("test-only derivative can be written");
    let (report, _) = report(&path);
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        evidence(finding(&report, "c2pa.manifest_store"), "manifest_count"),
        "2"
    );
    for id in ["c2pa.manifest_validation", "c2pa.hard_binding"] {
        let finding = finding(&report, id);
        assert_eq!(finding.status(), FindingStatus::Unknown, "{id}");
        assert_eq!(
            evidence(finding, "validation_time_basis"),
            "not_reproducible",
            "{id}"
        );
    }
}

#[test]
fn png_content_manifest_and_truncation_tampering_never_validate() {
    let temp = TempDirectory::new();

    let mut content_tamper =
        fs::read(fixture("generated/signed.png")).expect("fixture can be read");
    let idat_length = png_chunk(&content_tamper, b"IDAT")
        .expect("signed PNG has image data")
        .0
        .len();
    mutate_png_chunk(&mut content_tamper, b"IDAT", idat_length / 2);
    let content_path = temp.join("content-tamper.png");
    fs::write(&content_path, &content_tamper).expect("derivative can be written");
    let (content_report, _) = report(&content_path);
    assert_eq!(
        finding(&content_report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&content_report, "c2pa.hard_binding").status(),
        FindingStatus::Unknown
    );

    let mut manifest_tamper =
        fs::read(fixture("generated/signed.png")).expect("fixture can be read");
    mutate_png_chunk(&mut manifest_tamper, b"caBX", 100);
    let manifest_path = temp.join("manifest-tamper.png");
    fs::write(&manifest_path, &manifest_tamper).expect("derivative can be written");
    let (manifest_report, _) = report(&manifest_path);
    assert_ne!(
        finding(&manifest_report, "c2pa.manifest_validation").status(),
        FindingStatus::Present
    );

    let mut truncated = fs::read(fixture("generated/signed.png")).expect("fixture can be read");
    let (manifest_data, _) = png_chunk(&truncated, b"caBX").expect("manifest chunk exists");
    truncated.truncate(manifest_data.start + manifest_data.len() / 2);
    let truncated_path = temp.join("truncated.png");
    fs::write(&truncated_path, truncated).expect("derivative can be written");
    let (truncated_report, _) = report(&truncated_path);
    assert_eq!(
        finding(&truncated_report, "c2pa.manifest_store").status(),
        FindingStatus::Invalid
    );
}

#[test]
fn svg_content_manifest_and_truncation_tampering_never_validate() {
    let temp = TempDirectory::new();

    let mut content_tamper =
        fs::read(fixture("generated/signed.svg")).expect("fixture can be read");
    replace_equal_length(
        &mut content_tamper,
        b"This is an XMP test",
        b"This is an YMP test",
    );
    let content_path = temp.join("content-tamper.svg");
    fs::write(&content_path, content_tamper).expect("derivative can be written");
    let (content_report, _) = report(&content_path);
    assert_eq!(
        finding(&content_report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&content_report, "c2pa.hard_binding").status(),
        FindingStatus::Unknown
    );

    let mut manifest_tamper =
        fs::read(fixture("generated/signed.svg")).expect("fixture can be read");
    let manifest_start = manifest_tamper
        .windows(b"<c2pa:manifest>".len())
        .position(|window| window == b"<c2pa:manifest>")
        .expect("manifest element exists")
        + b"<c2pa:manifest>".len();
    let mutation = manifest_start + 500;
    manifest_tamper[mutation] = if manifest_tamper[mutation] == b'A' {
        b'B'
    } else {
        b'A'
    };
    let manifest_path = temp.join("manifest-tamper.svg");
    fs::write(&manifest_path, manifest_tamper).expect("derivative can be written");
    let (manifest_report, _) = report(&manifest_path);
    assert_ne!(
        finding(&manifest_report, "c2pa.manifest_validation").status(),
        FindingStatus::Present
    );

    let original = fs::read(fixture("generated/signed.svg")).expect("fixture can be read");
    let manifest_end = original
        .windows(b"</c2pa:manifest>".len())
        .position(|window| window == b"</c2pa:manifest>")
        .expect("manifest element closes");
    let mut truncated = original;
    truncated.drain(manifest_start + 16..manifest_end);
    let truncated_path = temp.join("truncated.svg");
    fs::write(&truncated_path, truncated).expect("derivative can be written");
    let (truncated_report, _) = report(&truncated_path);
    assert_eq!(
        finding(&truncated_report, "c2pa.manifest_store").status(),
        FindingStatus::Invalid
    );
}

#[test]
fn signed_svg_with_leading_arbitrary_processing_instruction_is_still_svg() {
    let temp = TempDirectory::new();
    let original = fs::read(fixture("generated/signed.svg")).expect("fixture can be read");
    let declaration_end = original
        .windows(b"?>".len())
        .position(|window| window == b"?>")
        .expect("signed SVG has an XML declaration")
        + b"?>".len();
    let mut bytes = original[..declaration_end].to_vec();
    bytes.extend_from_slice(b"\n<?audit fixed?>");
    bytes.extend_from_slice(&original[declaration_end..]);
    let path = temp.join("signed-with-pi.txt");
    fs::write(&path, bytes).expect("derivative can be written");

    let (report, _) = report(&path);
    assert_eq!(
        finding(&report, "c2pa.text_manifest_wrapper").status(),
        FindingStatus::NotApplicable
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_validation").status(),
        FindingStatus::Unknown
    );
    assert_eq!(
        finding(&report, "c2pa.hard_binding").status(),
        FindingStatus::Unknown
    );
}

#[test]
fn signed_svg_with_invalid_or_reserved_pi_target_is_not_classified_as_svg() {
    let original = fs::read(fixture("generated/signed.svg")).expect("fixture can be read");
    let temp = TempDirectory::new();
    for (name, prefix) in [
        ("digit-target.txt", b"<?1bad?>\n".as_slice()),
        ("reserved-target.txt", b"<?XML data?>\n".as_slice()),
    ] {
        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(&original);
        let path = temp.join(name);
        fs::write(&path, bytes).expect("test-only derivative can be written");
        let (report, _) = report(&path);
        assert_eq!(
            finding(&report, "c2pa.text_manifest_wrapper").status(),
            FindingStatus::Absent,
            "{name}"
        );
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::NotApplicable,
            "{name}"
        );
    }
}

#[test]
fn unsigned_supported_formats_are_absent_and_wrong_extensions_do_not_override_content() {
    for relative in ["unsigned/sample.png", "unsigned/sample.svg"] {
        let (report, _) = report(&fixture(relative));
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::Absent,
            "{relative}"
        );
    }

    let temp = TempDirectory::new();
    for (relative, misleading_name) in [
        ("public-testfiles/adobe-20220124-CA.jpg", "signed.txt"),
        ("generated/signed.svg", "signed.jpg"),
    ] {
        let path = temp.join(misleading_name);
        fs::copy(fixture(relative), &path).expect("fixture can be copied");
        let (report, _) = report(&path);
        assert_eq!(
            finding(&report, "c2pa.manifest_store").status(),
            FindingStatus::Present,
            "{relative}"
        );
    }
}

fn encode_wrapper(payload: &[u8]) -> Vec<u8> {
    let mut text = String::from('\u{feff}');
    let length = u32::try_from(payload.len()).expect("test payload fits u32");
    for byte in b"C2PATXT\0"
        .iter()
        .copied()
        .chain([1])
        .chain(length.to_be_bytes())
        .chain(payload.iter().copied())
    {
        let scalar = if byte < 16 {
            0xfe00 + u32::from(byte)
        } else {
            0xe0100 + u32::from(byte) - 16
        };
        text.push(char::from_u32(scalar).expect("mapping yields a scalar"));
    }
    text.into_bytes()
}

#[test]
fn a8_wrapper_crosses_the_real_read_boundary_and_coexists_with_unicode() {
    let temp = TempDirectory::new();
    let path = temp.join("boundary.txt");
    let mut bytes = vec![b'a'; 65_530];
    bytes.extend_from_slice(&encode_wrapper(&[0, 16, 255]));
    fs::write(&path, &bytes).expect("test artifact can be written");

    let (report, _) = report(&path);
    assert_eq!(
        finding(&report, "c2pa.text_manifest_wrapper").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "unicode.default_ignorable_code_point").status(),
        FindingStatus::Present
    );
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Unsupported
    );
    assert_eq!(fs::read(&path).expect("artifact remains readable"), bytes);
}

#[test]
fn malformed_utf8_text_wrapper_evaluation_is_invalid_not_absent() {
    let temp = TempDirectory::new();
    let path = temp.join("malformed.txt");
    let mut bytes = vec![0xff];
    bytes.extend_from_slice(&encode_wrapper(&[]));
    fs::write(&path, bytes).expect("test artifact can be written");
    let (report, _) = report(&path);
    assert_eq!(
        finding(&report, "c2pa.text_manifest_wrapper").status(),
        FindingStatus::Invalid
    );
}

#[test]
fn hostile_ambient_settings_and_neighbor_sidecar_have_zero_effect() {
    let temp = TempDirectory::new();
    let path = temp.join("victim.png");
    fs::copy(fixture("unsigned/sample.png"), &path).expect("fixture can be copied");
    fs::copy(
        fixture("public-testfiles/adobe-20220124-CA.jpg"),
        temp.join("victim.c2pa"),
    )
    .expect("hostile sidecar can be placed");

    let baseline = run(&path, true);
    assert!(baseline.status.success());
    let hostile = Command::new(env!("CARGO_BIN_EXE_scrub"))
        .arg("inspect")
        .arg(&path)
        .arg("--json")
        .env("C2PA_CONFIG", temp.join("hostile.toml"))
        .env("C2PA_SETTINGS", "verify.verify_after_reading=false")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .expect("scrub process can run");
    assert!(hostile.status.success());
    assert!(hostile.stderr.is_empty());
    assert_eq!(hostile.stdout, baseline.stdout);

    let report = Report::from_json(
        std::str::from_utf8(&hostile.stdout)
            .expect("stdout is UTF-8")
            .trim_end(),
    )
    .expect("stdout is an untrusted report")
    .into_report();
    assert_eq!(
        finding(&report, "c2pa.manifest_store").status(),
        FindingStatus::Absent
    );
}

#[test]
fn repeated_json_is_byte_identical_and_human_success_stderr_is_empty() {
    let path = fixture("public-testfiles/adobe-20220124-CA.jpg");
    let first = run(&path, true);
    let second = run(&path, true);
    assert!(first.status.success());
    assert!(second.status.success());
    assert!(first.stderr.is_empty());
    assert!(second.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout);

    let human = run(&path, false);
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    let output = String::from_utf8(human.stdout).expect("human stdout is UTF-8");
    assert!(output.contains("PRESENT         manifest store"));
    assert!(!output.contains("C2PA: yes"));
    assert!(!output.contains("AI detected"));
}
