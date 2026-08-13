use std::error::Error;
use std::fmt::{self, Write as _};

use scrub_report::{Evidence, Finding, FindingStatus, MechanismIdentity, Sha256Digest};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

pub(crate) const NFC_MECHANISM_ID: &str = "unicode.normalization.nfc_difference";
pub(crate) const NFKC_MECHANISM_ID: &str = "unicode.normalization.nfkc_difference";
pub(crate) const UNICODE_VERSION: &str = "17.0.0";

const RETAINED_SCALAR_LIMIT: usize = 8;
const NFC_LIMITATION: &str = "An NFC difference is a neutral Unicode normalization observation; it does not establish security risk, provenance, authorship, intent, or watermark presence.";
const NFKC_LIMITATION: &str = "An NFKC difference is a neutral Unicode compatibility-normalization observation; compatibility folding can erase distinctions and must not be interpreted as sanitization, security risk, provenance, authorship, intent, or watermark presence.";
const INVALID_UTF8_LIMITATION: &str =
    "Normalization evidence is unavailable because the complete artifact is not valid UTF-8.";
const INVALID_UTF8_EVIDENCE: &str =
    "failed: the complete artifact is malformed or incomplete UTF-8";

pub(crate) fn valid_findings(input: &str) -> Result<[Finding; 2], AnalysisError> {
    Ok([
        analyze(input, NFC_MECHANISM_ID, NFC_LIMITATION, || input.nfc())?,
        analyze(input, NFKC_MECHANISM_ID, NFKC_LIMITATION, || input.nfkc())?,
    ])
}

#[allow(dead_code)] // Some integration targets include this module without the C2PA caller.
pub(crate) fn nfc_utf8_byte_length(input: &str) -> Result<u64, AnalysisError> {
    Ok(normalized_summary(input.nfc())?.byte_length)
}

pub(crate) fn invalid_findings() -> [Finding; 2] {
    [
        invalid_finding(NFC_MECHANISM_ID),
        invalid_finding(NFKC_MECHANISM_ID),
    ]
}

fn invalid_finding(mechanism_id: &str) -> Finding {
    Finding::new(
        MechanismIdentity::new(mechanism_id, UNICODE_VERSION),
        FindingStatus::Invalid,
        vec![Evidence::new("utf8_validation", INVALID_UTF8_EVIDENCE)],
        vec![INVALID_UTF8_LIMITATION.to_owned()],
        vec![],
    )
}

fn analyze<I>(
    input: &str,
    mechanism_id: &str,
    limitation: &str,
    normalize: impl Fn() -> I,
) -> Result<Finding, AnalysisError>
where
    I: Iterator<Item = char>,
{
    let summary = normalized_summary(normalize())?;
    let difference = first_difference(input, normalize())?;
    let (status, evidence) = match difference {
        None => (FindingStatus::Absent, vec![]),
        Some(position) => (
            FindingStatus::Present,
            vec![
                Evidence::new(
                    "first_difference",
                    first_difference_json(input, normalize(), position)?,
                ),
                Evidence::new("normalized_byte_length", summary.byte_length.to_string()),
                Evidence::new("normalized_scalar_count", summary.scalar_count.to_string()),
                Evidence::new("normalized_sha256", summary.sha256.to_string()),
            ],
        ),
    };

    Ok(Finding::new(
        MechanismIdentity::new(mechanism_id, UNICODE_VERSION),
        status,
        evidence,
        vec![limitation.to_owned()],
        vec![],
    ))
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct NormalizedSummary {
    sha256: Sha256Digest,
    byte_length: u64,
    scalar_count: u64,
}

fn normalized_summary(
    normalized: impl Iterator<Item = char>,
) -> Result<NormalizedSummary, AnalysisError> {
    let mut hasher = Sha256::new();
    let mut byte_length = 0_u64;
    let mut scalar_count = 0_u64;
    let mut encoded = [0_u8; 4];

    for scalar in normalized {
        let bytes = scalar.encode_utf8(&mut encoded).as_bytes();
        hasher.update(bytes);
        byte_length = byte_length
            .checked_add(u64::try_from(bytes.len()).map_err(|_| AnalysisError::ArithmeticOverflow)?)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
        scalar_count = scalar_count
            .checked_add(1)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
    }

    let digest: [u8; 32] = hasher.finalize().into();
    Ok(NormalizedSummary {
        sha256: Sha256Digest::from_bytes(digest),
        byte_length,
        scalar_count,
    })
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DifferencePosition {
    scalar_index: u64,
    original_byte_offset: u64,
}

fn first_difference(
    input: &str,
    mut normalized: impl Iterator<Item = char>,
) -> Result<Option<DifferencePosition>, AnalysisError> {
    let mut original = input.char_indices();
    let mut scalar_index = 0_u64;

    loop {
        let original_next = original.next();
        let normalized_next = normalized.next();
        match (original_next, normalized_next) {
            (None, None) => return Ok(None),
            (Some((_, original)), Some(normalized)) if original == normalized => {
                scalar_index = scalar_index
                    .checked_add(1)
                    .ok_or(AnalysisError::ArithmeticOverflow)?;
            }
            (original, _) => {
                let original_byte_offset =
                    match original {
                        Some((byte_offset, _)) => u64::try_from(byte_offset)
                            .map_err(|_| AnalysisError::ArithmeticOverflow)?,
                        None => u64::try_from(input.len())
                            .map_err(|_| AnalysisError::ArithmeticOverflow)?,
                    };
                return Ok(Some(DifferencePosition {
                    scalar_index,
                    original_byte_offset,
                }));
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ScalarWindow {
    at_end: bool,
    scalars: Vec<String>,
    truncated: bool,
}

fn first_difference_json(
    input: &str,
    normalized: impl Iterator<Item = char>,
    position: DifferencePosition,
) -> Result<String, AnalysisError> {
    let original = scalar_window(input.chars(), position.scalar_index)?;
    let normalized = scalar_window(normalized, position.scalar_index)?;
    let mut value = String::new();
    write!(
        value,
        "{{\"scalar_index\":{},\"original_byte_offset\":{},\"original\":",
        position.scalar_index, position.original_byte_offset
    )
    .expect("writing to a String cannot fail");
    write_window(&mut value, &original);
    value.push_str(",\"normalized\":");
    write_window(&mut value, &normalized);
    value.push('}');
    Ok(value)
}

fn scalar_window(
    mut scalars: impl Iterator<Item = char>,
    mut scalar_index: u64,
) -> Result<ScalarWindow, AnalysisError> {
    while scalar_index != 0 {
        if scalars.next().is_none() {
            return Ok(ScalarWindow {
                at_end: true,
                scalars: vec![],
                truncated: false,
            });
        }
        scalar_index = scalar_index
            .checked_sub(1)
            .ok_or(AnalysisError::ArithmeticOverflow)?;
    }

    let Some(first) = scalars.next() else {
        return Ok(ScalarWindow {
            at_end: true,
            scalars: vec![],
            truncated: false,
        });
    };

    let mut retained = Vec::with_capacity(RETAINED_SCALAR_LIMIT);
    retained.push(canonical_code_point(first));
    for _ in 1..RETAINED_SCALAR_LIMIT {
        let Some(scalar) = scalars.next() else {
            return Ok(ScalarWindow {
                at_end: false,
                scalars: retained,
                truncated: false,
            });
        };
        retained.push(canonical_code_point(scalar));
    }
    Ok(ScalarWindow {
        at_end: false,
        scalars: retained,
        truncated: scalars.next().is_some(),
    })
}

fn write_window(output: &mut String, window: &ScalarWindow) {
    write!(output, "{{\"at_end\":{},\"scalars\":[", window.at_end)
        .expect("writing to a String cannot fail");
    for (index, scalar) in window.scalars.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(output, "\"{scalar}\"").expect("writing to a String cannot fail");
    }
    write!(output, "],\"truncated\":{}}}", window.truncated)
        .expect("writing to a String cannot fail");
}

fn canonical_code_point(value: char) -> String {
    let code_point = u32::from(value);
    if code_point <= 0xFFFF {
        format!("U+{code_point:04X}")
    } else {
        format!("U+{code_point:X}")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum AnalysisError {
    ArithmeticOverflow,
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => {
                formatter.write_str("normalization length or offset exceeds u64")
            }
        }
    }
}

impl Error for AnalysisError {}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ConformanceForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

#[cfg(test)]
pub(crate) fn normalize_code_points(form: ConformanceForm, input: &[u32]) -> Vec<u32> {
    let scalars = input
        .iter()
        .map(|code_point| char::from_u32(*code_point).expect("conformance input is scalar"));
    match form {
        ConformanceForm::Nfc => scalars.nfc().map(u32::from).collect(),
        ConformanceForm::Nfd => scalars.nfd().map(u32::from).collect(),
        ConformanceForm::Nfkc => scalars.nfkc().map(u32::from).collect(),
        ConformanceForm::Nfkd => scalars.nfkd().map(u32::from).collect(),
    }
}

#[cfg(test)]
pub(crate) fn normalization_matches(
    form: ConformanceForm,
    input: &[u32],
    expected: &[u32],
) -> bool {
    let scalars = input
        .iter()
        .map(|code_point| char::from_u32(*code_point).expect("conformance input is scalar"));
    match form {
        ConformanceForm::Nfc => scalars.nfc().map(u32::from).eq(expected.iter().copied()),
        ConformanceForm::Nfd => scalars.nfd().map(u32::from).eq(expected.iter().copied()),
        ConformanceForm::Nfkc => scalars.nfkc().map(u32::from).eq(expected.iter().copied()),
        ConformanceForm::Nfkd => scalars.nfkd().map(u32::from).eq(expected.iter().copied()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_unicode_version_is_exactly_the_mechanism_version() {
        assert_eq!(unicode_normalization::UNICODE_VERSION, (17, 0, 0));
        assert_eq!(UNICODE_VERSION, "17.0.0");
        for form in [
            ConformanceForm::Nfc,
            ConformanceForm::Nfd,
            ConformanceForm::Nfkc,
            ConformanceForm::Nfkd,
        ] {
            assert!(normalization_matches(form, &[0x41], &[0x41]));
            assert_eq!(normalize_code_points(form, &[0x41]), [0x41]);
        }
    }

    #[test]
    fn absent_has_no_normalization_specific_evidence() {
        for finding in valid_findings("ASCII 世界 😀").expect("analysis succeeds") {
            assert_eq!(finding.status(), FindingStatus::Absent);
            assert!(finding.evidence().is_empty());
        }
    }

    #[test]
    fn invalid_has_only_frozen_utf8_evidence() {
        for finding in invalid_findings() {
            assert_eq!(finding.status(), FindingStatus::Invalid);
            assert_eq!(
                finding.evidence(),
                [Evidence::new("utf8_validation", INVALID_UTF8_EVIDENCE)]
            );
        }
    }

    #[test]
    fn first_difference_offsets_follow_ascii_bmp_and_supplementary_prefixes() {
        for (input, expected_index, expected_offset) in [
            ("abce\u{301}", 3, 3),
            ("\u{754c}e\u{301}", 1, 3),
            ("\u{1f600}e\u{301}", 1, 4),
        ] {
            let difference = first_difference(input, input.nfc())
                .expect("comparison succeeds")
                .expect("input differs under NFC");
            assert_eq!(difference.scalar_index, expected_index);
            assert_eq!(difference.original_byte_offset, expected_offset);
        }
    }

    #[test]
    fn first_difference_freezes_contraction_expansion_reordering_and_end_cases() {
        let contraction = finding_for(NFC_MECHANISM_ID, "e\u{301}");
        assert_eq!(
            evidence(&contraction, "first_difference"),
            r#"{"scalar_index":0,"original_byte_offset":0,"original":{"at_end":false,"scalars":["U+0065","U+0301"],"truncated":false},"normalized":{"at_end":false,"scalars":["U+00E9"],"truncated":false}}"#
        );

        let expansion = finding_for(NFKC_MECHANISM_ID, "\u{fb03}");
        assert_eq!(
            evidence(&expansion, "first_difference"),
            r#"{"scalar_index":0,"original_byte_offset":0,"original":{"at_end":false,"scalars":["U+FB03"],"truncated":false},"normalized":{"at_end":false,"scalars":["U+0066","U+0066","U+0069"],"truncated":false}}"#
        );

        let reordering = finding_for(NFC_MECHANISM_ID, "\u{301}\u{323}");
        assert_eq!(
            evidence(&reordering, "first_difference"),
            r#"{"scalar_index":0,"original_byte_offset":0,"original":{"at_end":false,"scalars":["U+0301","U+0323"],"truncated":false},"normalized":{"at_end":false,"scalars":["U+0323","U+0301"],"truncated":false}}"#
        );

        let original_ends = first_difference("ab", "abc".chars())
            .expect("comparison succeeds")
            .expect("sequences differ");
        assert_eq!(original_ends.scalar_index, 2);
        assert_eq!(original_ends.original_byte_offset, 2);
        assert!(
            scalar_window("ab".chars(), 2)
                .expect("window succeeds")
                .at_end
        );

        let normalized_ends = first_difference("abc", "ab".chars())
            .expect("comparison succeeds")
            .expect("sequences differ");
        assert_eq!(normalized_ends.scalar_index, 2);
        assert_eq!(normalized_ends.original_byte_offset, 2);
        assert!(
            scalar_window("ab".chars(), 2)
                .expect("window succeeds")
                .at_end
        );

        assert_eq!(
            first_difference_json("ab", "abc".chars(), original_ends).expect("evidence succeeds"),
            r#"{"scalar_index":2,"original_byte_offset":2,"original":{"at_end":true,"scalars":[],"truncated":false},"normalized":{"at_end":false,"scalars":["U+0063"],"truncated":false}}"#
        );
        assert_eq!(
            first_difference_json("abc", "ab".chars(), normalized_ends).expect("evidence succeeds"),
            r#"{"scalar_index":2,"original_byte_offset":2,"original":{"at_end":false,"scalars":["U+0063"],"truncated":false},"normalized":{"at_end":true,"scalars":[],"truncated":false}}"#
        );
    }

    #[test]
    fn scalar_windows_retain_exactly_eight_and_mark_nine_as_truncated() {
        let eight = scalar_window("abcdefgh".chars(), 0).expect("window succeeds");
        assert_eq!(eight.scalars.len(), 8);
        assert!(!eight.truncated);
        assert!(!eight.at_end);

        let nine = scalar_window("abcdefghi".chars(), 0).expect("window succeeds");
        assert_eq!(nine.scalars.len(), 8);
        assert!(nine.truncated);
        assert!(!nine.at_end);
    }

    fn finding_for(mechanism_id: &str, input: &str) -> Finding {
        valid_findings(input)
            .expect("analysis succeeds")
            .into_iter()
            .find(|finding| finding.mechanism().id() == mechanism_id)
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
}
