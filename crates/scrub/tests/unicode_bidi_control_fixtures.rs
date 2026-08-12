#[path = "support/unicode_bidi_control.rs"]
mod unicode_bidi_control;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use scrub_report::{Evidence, FindingStatus, Sha256Digest};
use sha2::{Digest, Sha256};
use unicode_bidi_control::{
    CodePointRange, ExpectedArtifactIdentity, ExpectedLocation, ExpectedObservation,
    INSPECTION_READ_BOUNDARY, RETAINED_LOCATION_LIMIT, control_identities, fixture_corpus,
    parse_bidi_control_ranges, pinned_bidi_control_ranges,
};

const PINNED_UCD_BYTE_LENGTH: usize = 145_465;
const PINNED_UCD_SHA256: &str = "130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd";
const PINNED_CANONICAL_SHA256: &str =
    "217873f8bf2ca674f32afff23b3dc5fd81e4b55b5f6aa978c63417ad29f22674";

#[test]
fn compact_membership_data_has_the_pinned_semantics() {
    let ranges = pinned_bidi_control_ranges();
    assert_eq!(ranges, pinned_bidi_control_ranges());
    assert_eq!(
        ranges,
        [
            CodePointRange {
                start: 0x061C,
                end: 0x061C,
            },
            CodePointRange {
                start: 0x200E,
                end: 0x200F,
            },
            CodePointRange {
                start: 0x202A,
                end: 0x202E,
            },
            CodePointRange {
                start: 0x2066,
                end: 0x2069,
            },
        ]
    );
    assert_eq!(ranges.len(), 4);
    assert_eq!(
        ranges
            .iter()
            .copied()
            .map(CodePointRange::code_point_count)
            .sum::<u32>(),
        12
    );
    assert!(
        ranges
            .windows(2)
            .all(|adjacent| adjacent[0].end < adjacent[1].start)
    );

    let mut canonical_ranges = String::new();
    for range in &ranges {
        writeln!(canonical_ranges, "{:06X}..{:06X}", range.start, range.end)
            .expect("writing to a String cannot fail");
    }
    assert_eq!(
        canonical_ranges,
        "00061C..00061C\n00200E..00200F\n00202A..00202E\n002066..002069\n"
    );
    assert_eq!(canonical_ranges.len(), 60);
    assert_sha256(canonical_ranges.as_bytes(), PINNED_CANONICAL_SHA256);
}

#[test]
fn membership_parser_rejects_malformed_property_records() {
    assert!(parse_bidi_control_ranges("Bidi_Control").is_err());
    assert!(parse_bidi_control_ranges("2069..2066 ; Bidi_Control").is_err());
    assert!(parse_bidi_control_ranges("not-hex ; Bidi_Control").is_err());
}

#[test]
fn compact_membership_data_agrees_with_full_pinned_ucd_when_available() {
    let path = full_pinned_ucd_path();
    if !path.is_file() {
        eprintln!(
            "skipping full UCD parity check because the local research file is absent: {}",
            path.display()
        );
        return;
    }

    let bytes = fs::read(&path).expect("the pinned UCD file can be read");
    assert_eq!(bytes.len(), PINNED_UCD_BYTE_LENGTH);
    assert_sha256(&bytes, PINNED_UCD_SHA256);
    let full_data = std::str::from_utf8(&bytes).expect("the pinned UCD file is UTF-8");
    let full_ranges =
        parse_bidi_control_ranges(full_data).expect("the pinned UCD property data parses");

    assert_eq!(full_ranges, pinned_bidi_control_ranges());
}

#[test]
fn abbreviation_table_is_exact_and_bijective_over_membership() {
    let expected = [
        (0x061C, "ALM"),
        (0x200E, "LRM"),
        (0x200F, "RLM"),
        (0x202A, "LRE"),
        (0x202B, "RLE"),
        (0x202C, "PDF"),
        (0x202D, "LRO"),
        (0x202E, "RLO"),
        (0x2066, "LRI"),
        (0x2067, "RLI"),
        (0x2068, "FSI"),
        (0x2069, "PDI"),
    ];
    let actual: Vec<_> = control_identities()
        .iter()
        .map(|identity| (identity.code_point, identity.abbreviation))
        .collect();
    assert_eq!(actual, expected);

    let ranges = pinned_bidi_control_ranges();
    let code_points: BTreeSet<_> = control_identities()
        .iter()
        .map(|identity| identity.code_point)
        .collect();
    let abbreviations: BTreeSet<_> = control_identities()
        .iter()
        .map(|identity| identity.abbreviation)
        .collect();
    assert_eq!(code_points.len(), 12);
    assert_eq!(abbreviations.len(), 12);
    assert!(
        code_points
            .iter()
            .all(|code_point| is_member(*code_point, &ranges))
    );
    assert!(
        ranges
            .iter()
            .flat_map(|range| range.start..=range.end)
            .all(|code_point| code_points.contains(&code_point))
    );
}

#[test]
fn fixture_corpus_is_deterministic_and_matches_the_test_oracle() {
    let ranges = pinned_bidi_control_ranges();
    let first_construction = fixture_corpus();
    assert_eq!(first_construction, fixture_corpus());
    let mut names = BTreeSet::new();
    let mut directly_exercised_identities = BTreeSet::new();

    for fixture in first_construction {
        assert!(names.insert(fixture.name), "duplicate fixture name");
        let original_bytes = fixture.input.clone();
        match &fixture.expected {
            ExpectedObservation::InvalidUtf8 => {
                assert_eq!(fixture.expected.status(), FindingStatus::Invalid);
                assert!(std::str::from_utf8(&fixture.input).is_err());
                assert!(fixture.expected.property_evidence().is_empty());
                assert!(fixture.frozen_artifact_identity.is_some());
            }
            ExpectedObservation::Valid(expected) => {
                let input = std::str::from_utf8(&fixture.input).unwrap_or_else(|error| {
                    panic!("{} must be valid UTF-8: {error}", fixture.name)
                });
                let all_locations = oracle_locations(input, &ranges);
                let total_occurrence_count =
                    u64::try_from(all_locations.len()).expect("fixture count fits u64");
                let retained_locations: Vec<_> = all_locations
                    .into_iter()
                    .take(RETAINED_LOCATION_LIMIT)
                    .collect();
                let status = if total_occurrence_count == 0 {
                    FindingStatus::Absent
                } else {
                    FindingStatus::Present
                };

                assert_eq!(expected.status, status, "{} status", fixture.name);
                assert_eq!(
                    expected.total_occurrence_count, total_occurrence_count,
                    "{} total count",
                    fixture.name
                );
                assert_eq!(
                    expected.locations, retained_locations,
                    "{} retained locations",
                    fixture.name
                );
                assert_eq!(
                    expected.locations_truncated,
                    total_occurrence_count
                        > u64::try_from(RETAINED_LOCATION_LIMIT).expect("limit fits u64"),
                    "{} truncation",
                    fixture.name
                );
                assert!(expected.locations.windows(2).all(|pair| {
                    pair[0].byte_offset < pair[1].byte_offset
                        && pair[0].scalar_offset < pair[1].scalar_offset
                }));

                if fixture.name.starts_with("identity_") {
                    directly_exercised_identities.extend(
                        expected
                            .locations
                            .iter()
                            .map(|location| location.code_point),
                    );
                }
                for evidence in expected.report_evidence() {
                    assert!(
                        evidence
                            .value()
                            .chars()
                            .all(|value| { !is_member(u32::from(value), &ranges) })
                    );
                }
            }
        }
        assert_eq!(
            fixture.input, original_bytes,
            "{} input changed",
            fixture.name
        );
    }

    let expected_names: BTreeSet<_> = [
        "empty_utf8",
        "plain_ascii",
        "benign_non_ascii_without_controls",
        "ordinary_arabic_without_controls",
        "ordinary_hebrew_without_controls",
        "mixed_rtl_ltr_digits_without_controls",
        "dicp_non_bidi_zwj",
        "identity_alm_at_beginning",
        "identity_lrm_in_middle",
        "identity_rlm_at_end",
        "identity_lre_repeated",
        "identity_rle_after_multibyte_prefix",
        "identity_pdf_after_supplementary_scalar",
        "identity_lro_after_ascii_prefix",
        "identity_rlo_only",
        "identity_lri_only",
        "identity_rli_only",
        "identity_fsi_only",
        "identity_pdi_only",
        "structure_lre_then_pdf",
        "structure_rle_then_pdf",
        "structure_lri_then_pdi",
        "structure_rli_then_pdi",
        "structure_fsi_then_pdi",
        "structure_isolate_with_override",
        "all_control_identities",
        "exactly_256_bidi_controls",
        "exactly_257_bidi_controls",
        "valid_control_crosses_read_boundary",
        "lone_continuation_byte",
        "truncated_multibyte_sequence_at_eof",
        "overlong_encoding",
        "utf8_surrogate_encoding",
        "valid_control_prefix_then_invalid_utf8",
        "malformed_utf8_crosses_read_boundary",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected_names);
    assert_eq!(directly_exercised_identities.len(), 12);
}

#[test]
fn expected_evidence_has_one_deterministic_human_safe_encoding() {
    let fixture = named_fixture("identity_pdf_after_supplementary_scalar");
    let ExpectedObservation::Valid(expected) = fixture.expected else {
        panic!("offset fixture must be valid");
    };

    assert_eq!(
        expected.report_evidence(),
        vec![
            Evidence::new(
                "locations",
                r#"[{"code_point":"U+202C","abbreviation":"PDF","byte_offset":4,"scalar_offset":1}]"#,
            ),
            Evidence::new("locations_truncated", "false"),
            Evidence::new("total_occurrence_count", "1"),
        ]
    );

    let all_controls = named_fixture("all_control_identities");
    let ExpectedObservation::Valid(all_expected) = all_controls.expected else {
        panic!("complete-membership fixture must be valid");
    };
    let locations_json = all_expected
        .report_evidence()
        .into_iter()
        .find(|evidence| evidence.name() == "locations")
        .expect("locations evidence exists")
        .value()
        .to_owned();
    for identity in control_identities() {
        let expected_identity = format!(
            "\"code_point\":\"U+{:04X}\",\"abbreviation\":\"{}\"",
            identity.code_point, identity.abbreviation
        );
        assert!(locations_json.contains(&expected_identity));
    }
}

#[test]
fn bidi_control_is_a_strict_dicp_subset_and_findings_remain_independent() {
    let bidi_ranges = pinned_bidi_control_ranges();
    let dicp_ranges = parse_test_property_ranges(
        include_str!("fixtures/unicode-default-ignorable-17.0.0.txt"),
        "Default_Ignorable_Code_Point",
    );
    assert!(
        control_identities()
            .iter()
            .all(|identity| is_member(identity.code_point, &dicp_ranges))
    );
    assert!(is_member(0x200D, &dicp_ranges));
    assert!(!is_member(0x200D, &bidi_ranges));

    let non_bidi_dicp = named_fixture("dicp_non_bidi_zwj");
    assert_eq!(non_bidi_dicp.expected.status(), FindingStatus::Absent);
    let all_controls = named_fixture("all_control_identities");
    let input = std::str::from_utf8(&all_controls.input).expect("fixture is valid UTF-8");
    assert_eq!(
        input
            .chars()
            .filter(|value| is_member(u32::from(*value), &dicp_ranges))
            .count(),
        12
    );
    assert_eq!(all_controls.expected.status(), FindingStatus::Present);
}

#[test]
fn bounded_evidence_retains_exactly_the_first_256_locations() {
    let exactly_256 = named_fixture("exactly_256_bidi_controls");
    let ExpectedObservation::Valid(expected_256) = exactly_256.expected else {
        panic!("256 fixture must be valid");
    };
    assert_eq!(expected_256.total_occurrence_count, 256);
    assert_eq!(expected_256.locations.len(), 256);
    assert!(!expected_256.locations_truncated);

    let exactly_257 = named_fixture("exactly_257_bidi_controls");
    let ExpectedObservation::Valid(expected_257) = exactly_257.expected else {
        panic!("257 fixture must be valid");
    };
    assert_eq!(expected_257.total_occurrence_count, 257);
    assert_eq!(expected_257.locations.len(), 256);
    assert!(expected_257.locations_truncated);
    assert_eq!(expected_257.locations, expected_256.locations);
    assert_eq!(
        expected_257.locations.last(),
        Some(&ExpectedLocation {
            code_point: 0x202E,
            abbreviation: "RLO",
            byte_offset: 765,
            scalar_offset: 255,
        })
    );
}

#[test]
fn valid_control_scalar_crosses_the_existing_read_boundary() {
    let fixture = named_fixture("valid_control_crosses_read_boundary");
    assert_eq!(INSPECTION_READ_BOUNDARY, 65_536);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY - 1], 0xe2);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY], 0x80);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY + 1], 0xae);
    assert_frozen_artifact_identity(&fixture);

    let ExpectedObservation::Valid(expected) = fixture.expected else {
        panic!("boundary fixture must be valid");
    };
    assert_eq!(expected.total_occurrence_count, 1);
    assert_eq!(
        expected.locations,
        [ExpectedLocation {
            code_point: 0x202E,
            abbreviation: "RLO",
            byte_offset: 65_535,
            scalar_offset: 65_535,
        }]
    );
}

#[test]
fn every_invalid_fixture_has_frozen_identity_and_no_prefix_property_evidence() {
    let invalid_names = [
        "lone_continuation_byte",
        "truncated_multibyte_sequence_at_eof",
        "overlong_encoding",
        "utf8_surrogate_encoding",
        "valid_control_prefix_then_invalid_utf8",
        "malformed_utf8_crosses_read_boundary",
    ];
    for name in invalid_names {
        let fixture = named_fixture(name);
        assert_eq!(fixture.expected.status(), FindingStatus::Invalid);
        assert!(fixture.expected.property_evidence().is_empty());
        assert!(std::str::from_utf8(&fixture.input).is_err());
        assert_frozen_artifact_identity(&fixture);
    }
}

#[test]
fn malformed_utf8_crossing_boundary_discards_prefix_and_preserves_full_identity() {
    let fixture = named_fixture("malformed_utf8_crosses_read_boundary");
    assert_eq!(&fixture.input[..3], "\u{202e}".as_bytes());
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY - 1], 0xe2);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY], 0x28);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY + 1], 0xa1);
    assert!(fixture.input.len() > INSPECTION_READ_BOUNDARY * 2);
    let error = std::str::from_utf8(&fixture.input).expect_err("fixture is malformed UTF-8");
    assert_eq!(error.valid_up_to(), INSPECTION_READ_BOUNDARY - 1);
    assert_eq!(fixture.expected.status(), FindingStatus::Invalid);
    assert!(fixture.expected.property_evidence().is_empty());
    assert_frozen_artifact_identity(&fixture);
}

fn oracle_locations(input: &str, ranges: &[CodePointRange]) -> Vec<ExpectedLocation> {
    input
        .char_indices()
        .enumerate()
        .filter_map(|(scalar_offset, (byte_offset, value))| {
            let code_point = u32::from(value);
            if !is_member(code_point, ranges) {
                return None;
            }
            let abbreviation = control_identities()
                .iter()
                .find(|identity| identity.code_point == code_point)
                .expect("every member has one abbreviation")
                .abbreviation;
            Some(ExpectedLocation {
                code_point,
                abbreviation,
                byte_offset: u64::try_from(byte_offset).expect("fixture offset fits u64"),
                scalar_offset: u64::try_from(scalar_offset).expect("fixture offset fits u64"),
            })
        })
        .collect()
}

fn is_member(code_point: u32, ranges: &[CodePointRange]) -> bool {
    ranges
        .iter()
        .copied()
        .any(|range| range.contains(code_point))
}

fn named_fixture(name: &str) -> unicode_bidi_control::Fixture {
    fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == name)
        .unwrap_or_else(|| panic!("fixture {name} exists"))
}

fn assert_frozen_artifact_identity(fixture: &unicode_bidi_control::Fixture) {
    let ExpectedArtifactIdentity {
        byte_length,
        sha256,
    } = fixture
        .frozen_artifact_identity
        .expect("fixture has a frozen artifact identity");
    assert_eq!(
        u64::try_from(fixture.input.len()).expect("fixture length fits u64"),
        byte_length
    );
    assert_sha256(&fixture.input, sha256);
}

fn assert_sha256(bytes: &[u8], expected: &str) {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    assert_eq!(Sha256Digest::from_bytes(digest).as_str(), expected);
}

fn parse_test_property_ranges(input: &str, property_name: &str) -> Vec<CodePointRange> {
    input
        .lines()
        .filter_map(|line| {
            let (code_points, property) = line.split('#').next()?.split_once(';')?;
            if property.trim() != property_name {
                return None;
            }
            let code_points = code_points.trim();
            let (start, end) = code_points
                .split_once("..")
                .unwrap_or((code_points, code_points));
            Some(CodePointRange {
                start: u32::from_str_radix(start.trim(), 16).expect("fixture start is hex"),
                end: u32::from_str_radix(end.trim(), 16).expect("fixture end is hex"),
            })
        })
        .collect()
}

fn full_pinned_ucd_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/library/unicode/17.0.0/PropList.txt")
}
