use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Write as _};
use std::io::Cursor;

use c2pa::validation_results::{StatusCodes, ValidationState};
use c2pa::validation_status;
use c2pa::{Context, Reader, Settings};
use scrub_report::{Evidence, Finding, FindingStatus, MechanismIdentity, Sha256Digest};
use sha2::{Digest, Sha256};

use crate::unicode_normalization;

pub(crate) const VERSION: &str = "2.4";
pub(crate) const TEXT_WRAPPER_ID: &str = "c2pa.text_manifest_wrapper";
pub(crate) const MANIFEST_STORE_ID: &str = "c2pa.manifest_store";
pub(crate) const MANIFEST_VALIDATION_ID: &str = "c2pa.manifest_validation";
pub(crate) const HARD_BINDING_ID: &str = "c2pa.hard_binding";
pub(crate) const CREDENTIAL_TRUST_ID: &str = "c2pa.credential_trust";

const MAGIC: [u8; 8] = *b"C2PATXT\0";
const HEADER_BYTES: usize = 13;
const STATUS_LIMIT: usize = 256;
const CERTIFICATE_STATUS_LABEL: &str = "c2pa.certificate-status";
const INVALID_UTF8_EVIDENCE: &str =
    "failed: the complete artifact is malformed or incomplete UTF-8";
const TEXT_LIMITATION: &str = "C2PA 2.4 Appendix A.8 remains under review and may change; this carrier is distinct from Claude embedded text watermarking.";
const BINARY_LIMITATION: &str = "C2PA evidence describes a signed claim and its binding; it does not by itself establish authorship, truth, or whether a person or AI created the content.";
const A8_SDK_LIMITATION: &str = "Released c2pa-rs 0.90.12 has no stable Appendix A.8 asset handler; cryptographic manifest and hard-binding validation are unsupported without using an experimental handler or applying the wrong asset semantics.";
const TRUST_LIMITATION: &str = "Cryptographic integrity and signer trust are separate; scrub v0.1 configures no C2PA trust list or ambient trust roots.";

const SETTINGS: &str = r#"{
  "core": {
    "allowed_network_hosts": [],
    "decode_identity_assertions": false
  },
  "verify": {
    "verify_after_reading": true,
    "verify_after_sign": false,
    "verify_trust": false,
    "verify_timestamp_trust": false,
    "ocsp_fetch": false,
    "remote_manifest_fetch": false,
    "skip_ingredient_conflict_resolution": false,
    "strict_v1_validation": false
  },
  "trust": {
    "verify_trust_list": false,
    "user_anchors": null,
    "trust_anchors": null,
    "trust_config": null,
    "allowed_list": null
  },
  "cawg_trust": {
    "verify_trust_list": false,
    "user_anchors": null,
    "trust_anchors": null,
    "trust_config": null,
    "allowed_list": null
  }
}"#;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum BinaryFormat {
    Png,
    Jpeg,
}

impl BinaryFormat {
    fn sdk_format(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }
}

pub(crate) fn detect_binary(prefix: &[u8]) -> Option<BinaryFormat> {
    if prefix.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(BinaryFormat::Png)
    } else if prefix.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(BinaryFormat::Jpeg)
    } else {
        None
    }
}

pub(crate) fn valid_text_findings(input: &str) -> Result<[Finding; 5], AnalysisError> {
    if is_svg(input.as_bytes()) {
        binary_findings(BinaryFormatOrSvg::Svg, input.as_bytes())
    } else {
        text_findings(input)
    }
}

pub(crate) fn malformed_text_findings() -> [Finding; 5] {
    [
        finding(
            TEXT_WRAPPER_ID,
            FindingStatus::Invalid,
            vec![Evidence::new("utf8_validation", INVALID_UTF8_EVIDENCE)],
            &[TEXT_LIMITATION],
        ),
        not_applicable(MANIFEST_STORE_ID),
        not_applicable(MANIFEST_VALIDATION_ID),
        not_applicable(HARD_BINDING_ID),
        not_applicable(CREDENTIAL_TRUST_ID),
    ]
}

pub(crate) fn unsupported_findings() -> [Finding; 5] {
    [
        unsupported(TEXT_WRAPPER_ID, TEXT_LIMITATION),
        unsupported(MANIFEST_STORE_ID, BINARY_LIMITATION),
        unsupported(MANIFEST_VALIDATION_ID, BINARY_LIMITATION),
        unsupported(HARD_BINDING_ID, BINARY_LIMITATION),
        unsupported(CREDENTIAL_TRUST_ID, TRUST_LIMITATION),
    ]
}

pub(crate) fn binary_format_findings(
    format: BinaryFormat,
    bytes: &[u8],
) -> Result<[Finding; 5], AnalysisError> {
    binary_findings(BinaryFormatOrSvg::Binary(format), bytes)
}

fn text_findings(input: &str) -> Result<[Finding; 5], AnalysisError> {
    let inspection = inspect_wrappers(input)?;
    let wrapper = match inspection.first_error {
        Some(error) => finding(
            TEXT_WRAPPER_ID,
            FindingStatus::Invalid,
            vec![
                Evidence::new("candidate_count", inspection.candidate_count.to_string()),
                Evidence::new("first_error", error.to_json()),
            ],
            &[TEXT_LIMITATION],
        ),
        None if inspection.wrapper_count == 0 => finding(
            TEXT_WRAPPER_ID,
            FindingStatus::Absent,
            vec![],
            &[TEXT_LIMITATION],
        ),
        None if inspection.wrapper_count == 1 => {
            let first = inspection
                .first_wrapper
                .expect("a positive wrapper count retains the first wrapper");
            finding(
                TEXT_WRAPPER_ID,
                FindingStatus::Present,
                vec![
                    Evidence::new("first_wrapper", first.to_json()),
                    Evidence::new("wrapper_count", inspection.wrapper_count.to_string()),
                ],
                &[TEXT_LIMITATION],
            )
        }
        None => unreachable!("multiple valid wrappers retain the normative failure code"),
    };

    if wrapper.status() == FindingStatus::Present {
        Ok([
            wrapper,
            unsupported(MANIFEST_STORE_ID, A8_SDK_LIMITATION),
            unsupported(MANIFEST_VALIDATION_ID, A8_SDK_LIMITATION),
            unsupported(HARD_BINDING_ID, A8_SDK_LIMITATION),
            unsupported(CREDENTIAL_TRUST_ID, A8_SDK_LIMITATION),
        ])
    } else {
        Ok([
            wrapper,
            not_applicable(MANIFEST_STORE_ID),
            not_applicable(MANIFEST_VALIDATION_ID),
            not_applicable(HARD_BINDING_ID),
            not_applicable(CREDENTIAL_TRUST_ID),
        ])
    }
}

#[derive(Debug, Clone, Copy)]
enum BinaryFormatOrSvg {
    Binary(BinaryFormat),
    Svg,
}

impl BinaryFormatOrSvg {
    fn sdk_format(self) -> &'static str {
        match self {
            Self::Binary(format) => format.sdk_format(),
            Self::Svg => "image/svg+xml",
        }
    }
}

fn binary_findings(format: BinaryFormatOrSvg, bytes: &[u8]) -> Result<[Finding; 5], AnalysisError> {
    let wrapper = not_applicable(TEXT_WRAPPER_ID);
    let manifest_bytes = match c2pa::jumbf_io::load_jumbf_from_memory(format.sdk_format(), bytes) {
        Ok(bytes) => bytes,
        Err(c2pa::Error::JumbfNotFound | c2pa::Error::ProvenanceMissing) => {
            return Ok([
                wrapper,
                finding(
                    MANIFEST_STORE_ID,
                    FindingStatus::Absent,
                    vec![],
                    &[BINARY_LIMITATION],
                ),
                not_applicable(MANIFEST_VALIDATION_ID),
                not_applicable(HARD_BINDING_ID),
                not_applicable(CREDENTIAL_TRUST_ID),
            ]);
        }
        Err(error) if is_resource_or_external_error(&error) => {
            return Err(AnalysisError::SdkResource(error.to_string()));
        }
        Err(_) => {
            return Ok([
                wrapper,
                finding(
                    MANIFEST_STORE_ID,
                    FindingStatus::Invalid,
                    vec![],
                    &[BINARY_LIMITATION],
                ),
                not_applicable(MANIFEST_VALIDATION_ID),
                not_applicable(HARD_BINDING_ID),
                not_applicable(CREDENTIAL_TRUST_ID),
            ]);
        }
    };

    let parse_context = sdk_context(false)?;
    let reader = match Reader::from_context(parse_context)
        .with_stream(format.sdk_format(), Cursor::new(bytes))
    {
        Ok(reader) => reader,
        Err(error) if is_resource_or_external_error(&error) => {
            return Err(AnalysisError::SdkResource(error.to_string()));
        }
        Err(_) => {
            return Ok([
                wrapper,
                finding(
                    MANIFEST_STORE_ID,
                    FindingStatus::Invalid,
                    vec![],
                    &[BINARY_LIMITATION],
                ),
                not_applicable(MANIFEST_VALIDATION_ID),
                not_applicable(HARD_BINDING_ID),
                not_applicable(CREDENTIAL_TRUST_ID),
            ]);
        }
    };

    let active = match reader.active_manifest() {
        Some(active) => active,
        None => {
            return Ok([
                wrapper,
                finding(
                    MANIFEST_STORE_ID,
                    FindingStatus::Invalid,
                    vec![],
                    &[BINARY_LIMITATION],
                ),
                not_applicable(MANIFEST_VALIDATION_ID),
                not_applicable(HARD_BINDING_ID),
                not_applicable(CREDENTIAL_TRUST_ID),
            ]);
        }
    };

    let manifest_digest = sha256(&manifest_bytes);
    let store = finding(
        MANIFEST_STORE_ID,
        FindingStatus::Present,
        vec![
            Evidence::new(
                "claim_version",
                active
                    .claim_version()
                    .map_or_else(|| "unknown".to_owned(), |version| version.to_string()),
            ),
            Evidence::new("manifest_count", reader.manifests().len().to_string()),
            Evidence::new("manifest_store_sha256", manifest_digest.to_string()),
        ],
        &[BINARY_LIMITATION],
    );

    if !has_reproducible_validation_basis(&reader) {
        return Ok([
            wrapper,
            store,
            finding(
                MANIFEST_VALIDATION_ID,
                FindingStatus::Unknown,
                vec![Evidence::new("validation_time_basis", "not_reproducible")],
                &[BINARY_LIMITATION],
            ),
            finding(
                HARD_BINDING_ID,
                FindingStatus::Unknown,
                vec![Evidence::new("validation_time_basis", "not_reproducible")],
                &[BINARY_LIMITATION],
            ),
            not_applicable(CREDENTIAL_TRUST_ID),
        ]);
    }

    let validation_context = sdk_context(true)?;
    let reader = match Reader::from_context(validation_context)
        .with_stream(format.sdk_format(), Cursor::new(bytes))
    {
        Ok(reader) => reader,
        Err(error) if is_resource_or_external_error(&error) => {
            return Err(AnalysisError::SdkResource(error.to_string()));
        }
        Err(_) => {
            return Ok([
                wrapper,
                store,
                deterministic_validation_failure(),
                not_applicable(HARD_BINDING_ID),
                not_applicable(CREDENTIAL_TRUST_ID),
            ]);
        }
    };

    let results = reader.validation_results();
    let active_codes = results.and_then(|results| results.active_manifest());
    let validation_state = reader.validation_state();
    let validation_status = match validation_state {
        ValidationState::Invalid => FindingStatus::Invalid,
        ValidationState::Valid | ValidationState::Trusted => FindingStatus::Present,
    };
    let public_state = match validation_state {
        ValidationState::Invalid => "invalid",
        ValidationState::Valid => "valid",
        ValidationState::Trusted => "trusted",
    };
    let validation = finding(
        MANIFEST_VALIDATION_ID,
        validation_status,
        vec![
            Evidence::new("failure_codes", status_json(results, CodeKind::Failure)),
            Evidence::new(
                "informational_codes",
                status_json(results, CodeKind::Informational),
            ),
            Evidence::new("success_codes", status_json(results, CodeKind::Success)),
            Evidence::new("validation_state", public_state),
            Evidence::new("validation_time_basis", "validated_timestamp"),
        ],
        &[BINARY_LIMITATION],
    );
    let binding = hard_binding_finding(active_codes);
    let trust = if validation.status() == FindingStatus::Present {
        finding(
            CREDENTIAL_TRUST_ID,
            FindingStatus::Unknown,
            vec![Evidence::new("trust_policy", "not_configured")],
            &[TRUST_LIMITATION],
        )
    } else {
        not_applicable(CREDENTIAL_TRUST_ID)
    };

    Ok([wrapper, store, validation, binding, trust])
}

fn sdk_context(verify_after_reading: bool) -> Result<Context, AnalysisError> {
    let mut settings = Settings::new()
        .with_json(SETTINGS)
        .map_err(|error| AnalysisError::SdkConfiguration(error.to_string()))?;
    settings.verify.verify_after_reading = verify_after_reading;
    Context::new()
        .with_settings(settings)
        .map_err(|error| AnalysisError::SdkConfiguration(error.to_string()))
}

fn has_reproducible_validation_basis(reader: &Reader) -> bool {
    let Some(active_label) = reader.active_label() else {
        return false;
    };
    let mut pending = vec![active_label.to_owned()];
    let mut covered = HashSet::new();

    while let Some(label) = pending.pop() {
        if !covered.insert(label.clone()) {
            continue;
        }
        let Some(manifest) = reader.get_manifest(&label) else {
            return false;
        };
        if manifest
            .signature_info()
            .is_none_or(|signature| signature.time.is_none())
            || manifest
                .assertions()
                .iter()
                .any(|assertion| assertion.label() == CERTIFICATE_STATUS_LABEL)
        {
            return false;
        }
        pending.extend(
            manifest
                .ingredients()
                .iter()
                .filter_map(|ingredient| ingredient.active_manifest().map(str::to_owned)),
        );
    }

    true
}

fn deterministic_validation_failure() -> Finding {
    finding(
        MANIFEST_VALIDATION_ID,
        FindingStatus::Invalid,
        vec![
            Evidence::new("validation_state", "invalid"),
            Evidence::new("validation_time_basis", "validated_timestamp"),
        ],
        &[BINARY_LIMITATION],
    )
}

fn hard_binding_finding(codes: Option<&StatusCodes>) -> Finding {
    let Some(codes) = codes else {
        return not_applicable(HARD_BINDING_ID);
    };

    let failure = codes
        .failure()
        .iter()
        .filter_map(|status| binding_code(status.code(), true))
        .min_by_key(|result| result.code);
    let success = codes
        .success()
        .iter()
        .filter_map(|status| binding_code(status.code(), false))
        .min_by_key(|result| result.code);
    let result = failure.or(success);
    match result {
        Some(result) => finding(
            HARD_BINDING_ID,
            if result.failure {
                FindingStatus::Invalid
            } else {
                FindingStatus::Present
            },
            vec![
                Evidence::new("algorithm", "sdk_selected"),
                Evidence::new("binding_type", result.binding_type),
                Evidence::new("validation_code", result.code),
            ],
            &[BINARY_LIMITATION],
        ),
        None => not_applicable(HARD_BINDING_ID),
    }
}

#[derive(Debug, Clone, Copy)]
struct BindingCode<'a> {
    code: &'a str,
    binding_type: &'static str,
    failure: bool,
}

fn binding_code(code: &str, failure: bool) -> Option<BindingCode<'_>> {
    let binding_type = match code {
        validation_status::ASSERTION_DATAHASH_MATCH
        | validation_status::ASSERTION_DATAHASH_MISMATCH
        | validation_status::ASSERTION_DATAHASH_MALFORMED => "data_hash",
        validation_status::ASSERTION_BOXHASH_MATCH
        | validation_status::ASSERTION_BOXHASH_MISMATCH
        | validation_status::ASSERTION_BOXESHASH_MALFORMED => "boxes_hash",
        validation_status::ASSERTION_BMFFHASH_MATCH
        | validation_status::ASSERTION_BMFFHASH_MISMATCH
        | validation_status::ASSERTION_BMFFHASH_MALFORMED => "bmff_hash",
        validation_status::ASSERTION_COLLECTIONHASH_MATCH
        | validation_status::ASSERTION_COLLECTIONHASH_MISMATCH => "collection_hash",
        _ => return None,
    };
    Some(BindingCode {
        code,
        binding_type,
        failure,
    })
}

#[derive(Debug, Clone, Copy)]
enum CodeKind {
    Success,
    Informational,
    Failure,
}

fn status_json(results: Option<&c2pa::ValidationResults>, kind: CodeKind) -> String {
    let mut codes = Vec::new();
    if let Some(results) = results {
        if let Some(active) = results.active_manifest() {
            extend_codes(&mut codes, active, kind);
        }
        if let Some(ingredients) = results.ingredient_deltas() {
            for ingredient in ingredients {
                extend_codes(&mut codes, ingredient.validation_deltas(), kind);
            }
        }
    }
    codes.sort_unstable();
    codes.dedup();
    let total = codes.len();
    codes.truncate(STATUS_LIMIT);
    let mut output = String::from("{\"codes\":[");
    for (index, code) in codes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write_json_string(&mut output, code);
    }
    write!(
        output,
        "],\"total\":{total},\"truncated\":{}}}",
        total > STATUS_LIMIT
    )
    .expect("writing to a String cannot fail");
    output
}

fn extend_codes<'a>(output: &mut Vec<&'a str>, codes: &'a StatusCodes, kind: CodeKind) {
    let statuses = match kind {
        CodeKind::Success => codes.success(),
        CodeKind::Informational => codes.informational(),
        CodeKind::Failure => codes.failure(),
    };
    output.extend(statuses.iter().map(|status| status.code()));
}

fn write_json_string(output: &mut String, input: &str) {
    output.push('"');
    for scalar in input.chars() {
        match scalar {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            scalar if scalar < '\u{20}' => {
                write!(output, "\\u{:04x}", u32::from(scalar))
                    .expect("writing to a String cannot fail");
            }
            scalar => output.push(scalar),
        }
    }
    output.push('"');
}

fn is_resource_or_external_error(error: &c2pa::Error) -> bool {
    matches!(
        error,
        c2pa::Error::InsufficientMemory
            | c2pa::Error::OtherError(_)
            | c2pa::Error::ThreadReceiveError
            | c2pa::Error::OperationCancelled
            | c2pa::Error::RemoteManifestFetch(_)
            | c2pa::Error::RemoteManifestUrl(_)
            | c2pa::Error::HttpError(_)
            | c2pa::Error::HttpResolverError(_)
    )
}

fn is_svg(bytes: &[u8]) -> bool {
    let mut input = bytes;
    if let Some(rest) = input.strip_prefix(b"\xef\xbb\xbf") {
        input = rest;
    }
    let mut xml_declaration_allowed = true;
    loop {
        input = trim_ascii_start(input);
        if xml_declaration_allowed && is_xml_declaration_start(input) {
            let Some(end) = xml_declaration_end(input) else {
                return false;
            };
            input = &input[end + 2..];
            xml_declaration_allowed = false;
        } else if input.starts_with(b"<?") {
            let Some(end) = processing_instruction_end(input) else {
                return false;
            };
            input = &input[end + 2..];
            xml_declaration_allowed = false;
        } else if input.starts_with(b"<!--") {
            let Some(end) = find_bytes(input, b"-->") else {
                return false;
            };
            input = &input[end + 3..];
            xml_declaration_allowed = false;
        } else if input.starts_with(b"<!DOCTYPE") {
            let Some(end) = doctype_end(input) else {
                return false;
            };
            input = &input[end + 1..];
            xml_declaration_allowed = false;
        } else {
            break;
        }
    }
    let Some(rest) = input.strip_prefix(b"<svg") else {
        return false;
    };
    rest.first()
        .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
}

fn processing_instruction_end(input: &[u8]) -> Option<usize> {
    let end = find_bytes(input, b"?>")?;
    let body = input.get(2..end)?;
    let target_end = body
        .iter()
        .position(|byte| is_xml_whitespace(*byte))
        .unwrap_or(body.len());
    let target = std::str::from_utf8(&body[..target_end]).ok()?;
    if !is_xml_name(target) || target.eq_ignore_ascii_case("xml") {
        return None;
    }
    Some(end)
}

fn is_xml_declaration_start(input: &[u8]) -> bool {
    input
        .strip_prefix(b"<?xml")
        .and_then(|rest| rest.first())
        .is_some_and(|byte| is_xml_whitespace(*byte))
}

fn xml_declaration_end(input: &[u8]) -> Option<usize> {
    let end = find_bytes(input, b"?>")?;
    let mut body = input.get(5..end)?;
    body = consume_required_xml_whitespace(body)?;
    body = consume_pseudo_attribute(body, b"version", PseudoAttribute::Version)?;

    if body.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        let after_space = consume_required_xml_whitespace(body)?;
        if after_space.starts_with(b"encoding") {
            body = consume_pseudo_attribute(after_space, b"encoding", PseudoAttribute::Encoding)?;
        }
    }
    if body.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        let after_space = consume_required_xml_whitespace(body)?;
        if after_space.starts_with(b"standalone") {
            body =
                consume_pseudo_attribute(after_space, b"standalone", PseudoAttribute::Standalone)?;
        }
    }
    while body.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        body = &body[1..];
    }
    body.is_empty().then_some(end)
}

#[derive(Clone, Copy)]
enum PseudoAttribute {
    Version,
    Encoding,
    Standalone,
}

fn consume_pseudo_attribute<'a>(
    input: &'a [u8],
    name: &[u8],
    kind: PseudoAttribute,
) -> Option<&'a [u8]> {
    let mut rest = input.strip_prefix(name)?;
    while rest.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        rest = &rest[1..];
    }
    rest = rest.strip_prefix(b"=")?;
    while rest.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        rest = &rest[1..];
    }
    let quote = *rest.first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    rest = &rest[1..];
    let value_end = rest.iter().position(|byte| *byte == quote)?;
    let value = &rest[..value_end];
    let valid = match kind {
        PseudoAttribute::Version => value
            .strip_prefix(b"1.")
            .is_some_and(|minor| !minor.is_empty() && minor.iter().all(u8::is_ascii_digit)),
        PseudoAttribute::Encoding => {
            value.first().is_some_and(u8::is_ascii_alphabetic)
                && value
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        }
        PseudoAttribute::Standalone => matches!(value, b"yes" | b"no"),
    };
    valid.then(|| &rest[value_end + 1..])
}

fn consume_required_xml_whitespace(mut input: &[u8]) -> Option<&[u8]> {
    let original_len = input.len();
    while input.first().is_some_and(|byte| is_xml_whitespace(*byte)) {
        input = &input[1..];
    }
    (input.len() < original_len).then_some(input)
}

fn is_xml_name(target: &str) -> bool {
    let mut scalars = target.chars();
    scalars.next().is_some_and(is_xml_name_start) && scalars.all(is_xml_name_char)
}

fn is_xml_name_start(scalar: char) -> bool {
    matches!(
        scalar,
        ':' | 'A'..='Z' | '_' | 'a'..='z'
            | '\u{c0}'..='\u{d6}'
            | '\u{d8}'..='\u{f6}'
            | '\u{f8}'..='\u{2ff}'
            | '\u{370}'..='\u{37d}'
            | '\u{37f}'..='\u{1fff}'
            | '\u{200c}'..='\u{200d}'
            | '\u{2070}'..='\u{218f}'
            | '\u{2c00}'..='\u{2fef}'
            | '\u{3001}'..='\u{d7ff}'
            | '\u{f900}'..='\u{fdcf}'
            | '\u{fdf0}'..='\u{fffd}'
            | '\u{10000}'..='\u{effff}'
    )
}

fn is_xml_name_char(scalar: char) -> bool {
    is_xml_name_start(scalar)
        || matches!(
            scalar,
            '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}'
        )
}

fn is_xml_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn doctype_end(input: &[u8]) -> Option<usize> {
    let mut bracket_depth = 0_u32;
    let mut quote = None;
    for (index, &byte) in input.iter().enumerate() {
        if let Some(delimiter) = quote {
            if byte == delimiter {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'[' => bracket_depth = bracket_depth.checked_add(1)?,
            b']' => bracket_depth = bracket_depth.checked_sub(1)?,
            b'>' if bracket_depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn trim_ascii_start(mut input: &[u8]) -> &[u8] {
    while input.first().is_some_and(u8::is_ascii_whitespace) {
        input = &input[1..];
    }
    input
}

fn find_bytes(input: &[u8], needle: &[u8]) -> Option<usize> {
    input
        .windows(needle.len())
        .position(|window| window == needle)
}

#[derive(Debug, Default)]
struct WrapperInspection {
    wrapper_count: u64,
    candidate_count: u64,
    first_wrapper: Option<Wrapper>,
    first_error: Option<WrapperError>,
}

#[derive(Debug)]
struct Wrapper {
    original_byte_offset: u64,
    original_byte_length: u64,
    normalized_byte_offset: u64,
    normalized_byte_length: u64,
    declared_manifest_length: u32,
    manifest_store_sha256: Sha256Digest,
}

impl Wrapper {
    fn to_json(&self) -> String {
        format!(
            "{{\"original_byte_offset\":{},\"original_byte_length\":{},\"normalized_byte_offset\":{},\"normalized_byte_length\":{},\"wrapper_version\":1,\"declared_manifest_length\":{},\"manifest_store_sha256\":\"{}\"}}",
            self.original_byte_offset,
            self.original_byte_length,
            self.normalized_byte_offset,
            self.normalized_byte_length,
            self.declared_manifest_length,
            self.manifest_store_sha256
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct WrapperError {
    code: &'static str,
    original_byte_offset: u64,
}

impl WrapperError {
    fn to_json(self) -> String {
        format!(
            "{{\"code\":\"{}\",\"original_byte_offset\":{}}}",
            self.code, self.original_byte_offset
        )
    }
}

fn inspect_wrappers(input: &str) -> Result<WrapperInspection, AnalysisError> {
    let mut inspection = WrapperInspection::default();
    let mut scalars = input.char_indices().peekable();
    while let Some((start, scalar)) = scalars.next() {
        if scalar != '\u{feff}' {
            continue;
        }

        let mut decoded_header = [0_u8; HEADER_BYTES];
        let mut selector_count = 0_usize;
        let mut run_end = start
            .checked_add(scalar.len_utf8())
            .ok_or(AnalysisError::ArithmeticOverflow)?;
        while let Some(&(offset, scalar)) = scalars.peek() {
            let Some(byte) = selector_byte(scalar) else {
                break;
            };
            scalars.next();
            if selector_count < HEADER_BYTES {
                decoded_header[selector_count] = byte;
            }
            selector_count = selector_count
                .checked_add(1)
                .ok_or(AnalysisError::ArithmeticOverflow)?;
            run_end = offset
                .checked_add(scalar.len_utf8())
                .ok_or(AnalysisError::ArithmeticOverflow)?;
        }

        let magic_prefix_len = selector_count.min(MAGIC.len());
        let exact_magic_prefix = magic_prefix_len != 0
            && decoded_header[..magic_prefix_len] == MAGIC[..magic_prefix_len];
        if selector_count < MAGIC.len() {
            if exact_magic_prefix {
                inspection.candidate_count = inspection
                    .candidate_count
                    .checked_add(1)
                    .ok_or(AnalysisError::ArithmeticOverflow)?;
                record_error(&mut inspection, start, "manifest.text.corruptedWrapper")?;
            }
            continue;
        }
        if decoded_header[..MAGIC.len()] != MAGIC {
            continue;
        }

        inspection.candidate_count = inspection
            .candidate_count
            .checked_add(1)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
        if selector_count < HEADER_BYTES || decoded_header[8] != 1 {
            record_error(&mut inspection, start, "manifest.text.corruptedWrapper")?;
            continue;
        }
        let declared = u32::from_be_bytes([
            decoded_header[9],
            decoded_header[10],
            decoded_header[11],
            decoded_header[12],
        ]);
        let declared_usize =
            usize::try_from(declared).map_err(|_| AnalysisError::ArithmeticOverflow)?;
        let required = HEADER_BYTES
            .checked_add(declared_usize)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
        if selector_count != required {
            record_error(&mut inspection, start, "manifest.text.corruptedWrapper")?;
            continue;
        }

        inspection.wrapper_count = inspection
            .wrapper_count
            .checked_add(1)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
        if inspection.wrapper_count > 1 {
            record_error(&mut inspection, start, "manifest.text.multipleWrappers")?;
        }
        if inspection.first_wrapper.is_none() {
            let manifest_digest = wrapper_payload_sha256(&input[start..run_end]);
            let original_byte_offset =
                u64::try_from(start).map_err(|_| AnalysisError::ArithmeticOverflow)?;
            let original_byte_length =
                u64::try_from(run_end - start).map_err(|_| AnalysisError::ArithmeticOverflow)?;
            let normalized_byte_offset =
                unicode_normalization::nfc_utf8_byte_length(&input[..start])
                    .map_err(AnalysisError::Normalization)?;
            inspection.first_wrapper = Some(Wrapper {
                original_byte_offset,
                original_byte_length,
                normalized_byte_offset,
                normalized_byte_length: original_byte_length,
                declared_manifest_length: declared,
                manifest_store_sha256: manifest_digest,
            });
        }
    }
    Ok(inspection)
}

fn record_error(
    inspection: &mut WrapperInspection,
    start: usize,
    code: &'static str,
) -> Result<(), AnalysisError> {
    if inspection.first_error.is_none() {
        inspection.first_error = Some(WrapperError {
            code,
            original_byte_offset: u64::try_from(start)
                .map_err(|_| AnalysisError::ArithmeticOverflow)?,
        });
    }
    Ok(())
}

fn wrapper_payload_sha256(wrapper: &str) -> Sha256Digest {
    let mut hasher = Sha256::new();
    for scalar in wrapper.chars().skip(1 + HEADER_BYTES) {
        hasher.update([selector_byte(scalar).expect("validated wrapper contains only selectors")]);
    }
    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn selector_byte(scalar: char) -> Option<u8> {
    let code_point = u32::from(scalar);
    match code_point {
        0xfe00..=0xfe0f => u8::try_from(code_point - 0xfe00).ok(),
        0xe0100..=0xe01ef => u8::try_from(code_point - 0xe0100 + 16).ok(),
        _ => None,
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn finding(
    id: &str,
    status: FindingStatus,
    evidence: Vec<Evidence>,
    limitations: &[&str],
) -> Finding {
    Finding::new(
        MechanismIdentity::new(id, VERSION),
        status,
        evidence,
        limitations
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        vec![],
    )
}

fn not_applicable(id: &str) -> Finding {
    finding(id, FindingStatus::NotApplicable, vec![], &[])
}

fn unsupported(id: &str, limitation: &str) -> Finding {
    finding(id, FindingStatus::Unsupported, vec![], &[limitation])
}

#[derive(Debug)]
pub(crate) enum AnalysisError {
    ArithmeticOverflow,
    Normalization(unicode_normalization::AnalysisError),
    SdkConfiguration(String),
    SdkResource(String),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => {
                formatter.write_str("C2PA offset or count exceeds usize/u64")
            }
            Self::Normalization(error) => {
                write!(formatter, "NFC offset calculation failed: {error}")
            }
            Self::SdkConfiguration(error) => {
                write!(formatter, "fixed C2PA SDK configuration failed: {error}")
            }
            Self::SdkResource(error) => write!(
                formatter,
                "C2PA SDK resource or external-access failure: {error}"
            ),
        }
    }
}

impl Error for AnalysisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Normalization(error) => Some(error),
            Self::ArithmeticOverflow | Self::SdkConfiguration(_) | Self::SdkResource(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_byte(byte: u8) -> char {
        if byte < 16 {
            char::from_u32(0xfe00 + u32::from(byte)).expect("literal mapping is scalar")
        } else {
            char::from_u32(0xe0100 + u32::from(byte) - 16).expect("literal mapping is scalar")
        }
    }

    fn independent_wrapper(payload: &[u8]) -> String {
        let mut output = String::from('\u{feff}');
        let length = u32::try_from(payload.len()).expect("test payload fits u32");
        for byte in MAGIC
            .into_iter()
            .chain([1])
            .chain(length.to_be_bytes())
            .chain(payload.iter().copied())
        {
            output.push(encode_byte(byte));
        }
        output
    }

    fn finding_by_id<'a>(findings: &'a [Finding], id: &str) -> &'a Finding {
        findings
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

    #[test]
    fn ordinary_unicode_and_wrong_magic_are_absent() {
        let mut wrong_magic = String::from('\u{feff}');
        for byte in *b"NOTC2PA!" {
            wrong_magic.push(encode_byte(byte));
        }
        for input in [
            "",
            "ASCII",
            "multilingual \u{754c}\u{1f600}",
            "\u{feff}",
            "\u{2764}\u{fe0f}",
            "\u{4e00}\u{e0100}",
            wrong_magic.as_str(),
        ] {
            let findings = valid_text_findings(input).expect("analysis succeeds");
            assert_eq!(
                finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
                FindingStatus::Absent
            );
        }
    }

    #[test]
    fn hand_frozen_smallest_wrapper_matches_the_normative_scalar_mapping() {
        let input = "\u{feff}\u{e0133}\u{e0122}\u{e0140}\u{e0131}\u{e0144}\u{e0148}\u{e0144}\u{fe00}\u{fe01}\u{fe00}\u{fe00}\u{fe00}\u{fe00}";
        let findings = valid_text_findings(input).expect("analysis succeeds");
        let wrapper = finding_by_id(&findings, TEXT_WRAPPER_ID);
        assert_eq!(wrapper.status(), FindingStatus::Present);
        assert_eq!(evidence(wrapper, "wrapper_count"), "1");
        assert!(evidence(wrapper, "first_wrapper").contains(
            "\"manifest_store_sha256\":\"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\""
        ));
    }

    #[test]
    fn wrapper_discovery_covers_start_middle_end_and_large_prefix() {
        let wrapper = independent_wrapper(&[0xa5]);
        for input in [
            format!("{wrapper}tail"),
            format!("prefix{wrapper}tail"),
            format!("prefix{wrapper}"),
            format!("{}{wrapper}", "a".repeat(200_000)),
        ] {
            let findings = valid_text_findings(&input).expect("analysis succeeds");
            assert_eq!(
                finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
                FindingStatus::Present
            );
        }
    }

    #[test]
    fn many_feff_and_long_nonmagic_selector_runs_remain_absent() {
        let many_feff = "\u{feff}".repeat(200_000);
        assert_eq!(
            finding_by_id(
                &valid_text_findings(&many_feff).expect("linear scan succeeds"),
                TEXT_WRAPPER_ID,
            )
            .status(),
            FindingStatus::Absent
        );

        let mut nonmagic = String::from('\u{feff}');
        nonmagic.extend(std::iter::repeat_n('\u{fe0f}', 300_000));
        assert_eq!(
            finding_by_id(
                &valid_text_findings(&nonmagic).expect("linear scan succeeds"),
                TEXT_WRAPPER_ID,
            )
            .status(),
            FindingStatus::Absent
        );
    }

    #[test]
    fn large_valid_wrapper_is_bounded_by_physical_selectors() {
        let payload = vec![0x5a; 1_048_576];
        let wrapper = independent_wrapper(&payload);
        let findings = valid_text_findings(&wrapper).expect("large wrapper analysis succeeds");
        let finding = finding_by_id(&findings, TEXT_WRAPPER_ID);
        assert_eq!(finding.status(), FindingStatus::Present);
        assert!(evidence(finding, "first_wrapper").contains(&format!(
            "\"manifest_store_sha256\":\"{}\"",
            sha256(&payload)
        )));
    }

    #[test]
    fn valid_wrapper_freezes_header_endianness_hash_and_offsets() {
        let payload = [0x00, 0x10, 0xff, 0x42];
        let wrapper = independent_wrapper(&payload);
        let input = format!("Ae\u{301}\u{1f600}{wrapper}tail");
        let findings = valid_text_findings(&input).expect("analysis succeeds");
        let finding = finding_by_id(&findings, TEXT_WRAPPER_ID);
        assert_eq!(finding.status(), FindingStatus::Present);
        assert_eq!(evidence(finding, "wrapper_count"), "1");
        let expected_hash = sha256(&payload);
        assert_eq!(
            evidence(finding, "first_wrapper"),
            format!(
                "{{\"original_byte_offset\":8,\"original_byte_length\":64,\"normalized_byte_offset\":7,\"normalized_byte_length\":64,\"wrapper_version\":1,\"declared_manifest_length\":4,\"manifest_store_sha256\":\"{expected_hash}\"}}"
            )
        );
    }

    #[test]
    fn normalized_offsets_are_nfc_utf8_bytes_not_raw_bytes_or_scalars() {
        let wrapper = independent_wrapper(&[]);
        for (prefix, raw, normalized) in [
            ("ASCII", 5, 5),
            ("\u{754c}", 3, 3),
            ("\u{1f600}", 4, 4),
            ("e\u{301}", 3, 2),
        ] {
            let input = format!("{prefix}{wrapper}");
            let findings = valid_text_findings(&input).expect("analysis succeeds");
            let evidence = evidence(finding_by_id(&findings, TEXT_WRAPPER_ID), "first_wrapper");
            assert!(evidence.contains(&format!("\"original_byte_offset\":{raw}")));
            assert!(evidence.contains(&format!("\"normalized_byte_offset\":{normalized}")));
        }
    }

    #[test]
    fn truncated_bad_version_bad_lengths_and_extra_payload_are_invalid() {
        let valid = independent_wrapper(&[1, 2]);
        let mut cases = vec![format!("\u{feff}{}", encode_byte(b'C'))];
        cases.push(valid.chars().take(1 + 8).collect());
        let mut bad_version = valid.clone();
        let replacement = encode_byte(2).to_string();
        let version_start = bad_version
            .char_indices()
            .nth(1 + 8)
            .expect("version exists")
            .0;
        let version_end = version_start
            + bad_version[version_start..]
                .chars()
                .next()
                .unwrap()
                .len_utf8();
        bad_version.replace_range(version_start..version_end, &replacement);
        cases.push(bad_version);
        cases.push(valid.chars().take(1 + 11).collect());
        cases.push(
            independent_wrapper(&[1, 2])
                .chars()
                .take(1 + HEADER_BYTES + 1)
                .collect(),
        );
        let mut extra = independent_wrapper(&[]);
        extra.push(encode_byte(1));
        cases.push(extra);
        for input in cases {
            let findings = valid_text_findings(&input).expect("analysis succeeds");
            assert_eq!(
                finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
                FindingStatus::Invalid
            );
        }
    }

    #[test]
    fn interruption_and_header_mutation_have_distinct_structural_meanings() {
        let interrupted = format!("\u{feff}{}x", encode_byte(b'C'));
        let findings = valid_text_findings(&interrupted).expect("analysis succeeds");
        let wrapper = finding_by_id(&findings, TEXT_WRAPPER_ID);
        assert_eq!(wrapper.status(), FindingStatus::Invalid);
        assert_eq!(evidence(wrapper, "candidate_count"), "1");

        let mut wrong_magic = independent_wrapper(&[]);
        let magic_start = wrong_magic.char_indices().nth(1).expect("magic exists").0;
        let magic_end = magic_start
            + wrong_magic[magic_start..]
                .chars()
                .next()
                .expect("magic scalar exists")
                .len_utf8();
        wrong_magic.replace_range(magic_start..magic_end, &encode_byte(b'X').to_string());
        let findings = valid_text_findings(&wrong_magic).expect("analysis succeeds");
        assert_eq!(
            finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
            FindingStatus::Absent
        );
    }

    #[test]
    fn payload_byte_mutation_preserves_carrier_shape_but_changes_payload_hash() {
        let first =
            valid_text_findings(&independent_wrapper(&[1, 2, 3])).expect("analysis succeeds");
        let second =
            valid_text_findings(&independent_wrapper(&[1, 2, 4])).expect("analysis succeeds");
        let first = finding_by_id(&first, TEXT_WRAPPER_ID);
        let second = finding_by_id(&second, TEXT_WRAPPER_ID);
        assert_eq!(first.status(), FindingStatus::Present);
        assert_eq!(second.status(), FindingStatus::Present);
        assert_ne!(
            evidence(first, "first_wrapper"),
            evidence(second, "first_wrapper")
        );
    }

    #[test]
    fn enormous_declared_length_never_allocates_from_the_declaration() {
        let mut input = String::from('\u{feff}');
        for byte in MAGIC.into_iter().chain([1]).chain(u32::MAX.to_be_bytes()) {
            input.push(encode_byte(byte));
        }
        let findings = valid_text_findings(&input).expect("bounded analysis succeeds");
        assert_eq!(
            finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
            FindingStatus::Invalid
        );
    }

    #[test]
    fn wrapper_quantity_is_zero_or_one_and_corruption_remains_invalid() {
        let valid = independent_wrapper(&[]);
        for (input, candidates) in [
            (format!("{valid}x{valid}"), "2"),
            (format!("{valid}x{valid}x{valid}"), "3"),
        ] {
            let findings = valid_text_findings(&input).expect("analysis succeeds");
            let wrapper = finding_by_id(&findings, TEXT_WRAPPER_ID);
            assert_eq!(wrapper.status(), FindingStatus::Invalid);
            assert_eq!(evidence(wrapper, "candidate_count"), candidates);
            assert!(
                evidence(wrapper, "first_error")
                    .contains("\"code\":\"manifest.text.multipleWrappers\"")
            );
            assert!(
                wrapper
                    .evidence()
                    .iter()
                    .all(|item| item.name() != "first_wrapper" && item.name() != "wrapper_count")
            );
        }

        let malformed = format!("\u{feff}{}", encode_byte(b'C'));
        for (input, candidates) in [
            (format!("{malformed}x{valid}"), "2"),
            (format!("{valid}x{malformed}"), "2"),
            (format!("{valid}x{valid}x{malformed}"), "3"),
        ] {
            let findings = valid_text_findings(&input).expect("analysis succeeds");
            let wrapper = finding_by_id(&findings, TEXT_WRAPPER_ID);
            assert_eq!(wrapper.status(), FindingStatus::Invalid);
            assert_eq!(evidence(wrapper, "candidate_count"), candidates);
        }
    }

    #[test]
    fn svg_classification_is_content_based_and_bounded() {
        for input in [
            "<svg></svg>",
            "  \n<svg></svg>",
            "<?xml version=\"1.0\"?><svg />",
            "<?audit fixed?><svg />",
            "<?one?><svg />",
            "<?one x?><svg />",
            "<?_x y?><svg />",
            "<?ns-like value?><svg />",
            "<?éclair fixed?><svg />",
            "<?a\u{0301} fixed?><svg />",
            "<?audit one?><?review two?><!--x--><svg />",
            "\u{feff} <?xml version=\"1.0\"?> <?audit fixed?> <!--x--> <svg />",
        ] {
            let findings = valid_text_findings(input).expect("analysis succeeds");
            assert_eq!(
                finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
                FindingStatus::NotApplicable
            );
        }
        for input in [
            "<?audit fixed?><not-svg/>",
            "<?audit fixed<svg />",
            "random <?audit fixed?> <svg />",
            "<?> <svg />",
            "<?  bad?><svg />",
            "<?1bad?><svg />",
            "<?-bad?><svg />",
            "<?.bad?><svg />",
            "<?\u{0301}bad?><svg />",
            "<?xml data?><svg />",
            "<?XML data?><svg />",
            "<?Xml data?><svg />",
            "<?xml?><svg />",
            "<?audit\u{000b}fixed?><svg />",
        ] {
            let findings = valid_text_findings(input).expect("analysis succeeds");
            assert_eq!(
                finding_by_id(&findings, TEXT_WRAPPER_ID).status(),
                FindingStatus::Absent,
                "{input}"
            );
        }
    }

    #[test]
    fn unsigned_supported_assets_are_absent_without_message_matching() {
        let png = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let findings =
            binary_format_findings(BinaryFormat::Png, &png).expect("typed SDK lookup completes");
        assert_eq!(
            finding_by_id(&findings, MANIFEST_STORE_ID).status(),
            FindingStatus::Absent
        );
    }
}
