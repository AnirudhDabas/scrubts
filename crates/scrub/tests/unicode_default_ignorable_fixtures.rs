mod support;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use scrub_report::{Evidence, FindingStatus, Sha256Digest};
use sha2::{Digest, Sha256};
use support::unicode_default_ignorable::{
    CodePointRange, ExpectedLocation, ExpectedObservation, INSPECTION_READ_BOUNDARY,
    RETAINED_LOCATION_LIMIT, fixture_corpus, parse_dicp_ranges, pinned_dicp_ranges,
};

const PINNED_UCD_BYTE_LENGTH: usize = 1_134_783;
const PINNED_UCD_SHA256: &str = "24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08";
const PINNED_DICP_CANONICAL_SHA256: &str =
    "5d2e0f0aaa2d84955d13925234b7f806a613e25f0ab0fc9666b32b9120a6a42c";

#[test]
fn compact_membership_data_has_the_pinned_extent() {
    let ranges = pinned_dicp_ranges();

    assert_eq!(ranges.len(), 27);
    assert_eq!(
        ranges
            .iter()
            .copied()
            .map(CodePointRange::code_point_count)
            .sum::<u32>(),
        4_174
    );
    for adjacent in ranges.windows(2) {
        assert!(
            adjacent[0].end < adjacent[1].start,
            "ranges must be sorted and non-overlapping: {adjacent:?}"
        );
    }

    let mut canonical_ranges = String::new();
    for range in &ranges {
        writeln!(canonical_ranges, "{:06X}..{:06X}", range.start, range.end)
            .expect("writing to a String cannot fail");
    }
    let digest: [u8; 32] = Sha256::digest(canonical_ranges.as_bytes()).into();
    assert_eq!(
        Sha256Digest::from_bytes(digest).as_str(),
        PINNED_DICP_CANONICAL_SHA256
    );
}

#[test]
fn compact_membership_data_agrees_with_the_full_pinned_ucd_when_available() {
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
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(Sha256Digest::from_bytes(digest).as_str(), PINNED_UCD_SHA256);
    let full_data = std::str::from_utf8(&bytes).expect("the pinned UCD file is UTF-8");
    let full_ranges = parse_dicp_ranges(full_data).expect("the pinned UCD property data parses");

    assert_eq!(full_ranges, pinned_dicp_ranges());
}

#[test]
fn fixture_corpus_is_self_consistent_and_matches_the_test_oracle() {
    let ranges = pinned_dicp_ranges();
    let fixtures = fixture_corpus();
    let mut names = BTreeSet::new();

    for fixture in fixtures {
        assert!(names.insert(fixture.name), "duplicate fixture name");
        match fixture.expected {
            ExpectedObservation::InvalidUtf8 => {
                assert_eq!(
                    ExpectedObservation::InvalidUtf8.status(),
                    FindingStatus::Invalid
                );
                assert!(
                    std::str::from_utf8(&fixture.input).is_err(),
                    "{} must contain malformed or incomplete UTF-8",
                    fixture.name
                );
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
                        > u64::try_from(RETAINED_LOCATION_LIMIT).expect("location limit fits u64"),
                    "{} truncation",
                    fixture.name
                );
                assert!(
                    expected.locations.windows(2).all(|pair| {
                        pair[0].byte_offset < pair[1].byte_offset
                            && pair[0].scalar_offset < pair[1].scalar_offset
                    }),
                    "{} locations must follow input order",
                    fixture.name
                );
            }
        }
    }

    assert_eq!(names.len(), 24);
}

#[test]
fn report_fixture_evidence_has_one_deterministic_encoding() {
    let fixture = fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == "multibyte_scalars_before_match")
        .expect("offset fixture exists");
    let ExpectedObservation::Valid(expected) = fixture.expected else {
        panic!("offset fixture must be valid");
    };

    assert_eq!(
        expected.report_evidence(),
        vec![
            Evidence::new(
                "locations",
                r#"[{"code_point":"U+200B","byte_offset":5,"scalar_offset":2}]"#,
            ),
            Evidence::new("locations_truncated", "false"),
            Evidence::new("total_occurrence_count", "1"),
        ]
    );
}

#[test]
fn boundary_fixture_splits_a_dicp_scalar_at_the_existing_read_size() {
    let fixture = fixture_corpus()
        .into_iter()
        .find(|fixture| fixture.name == "dicp_spans_64_kib_read_boundary")
        .expect("boundary fixture exists");
    let ExpectedObservation::Valid(expected) = fixture.expected else {
        panic!("boundary fixture must be valid");
    };

    assert_eq!(INSPECTION_READ_BOUNDARY, 65_536);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY - 1], 0xe2);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY], 0x80);
    assert_eq!(fixture.input[INSPECTION_READ_BOUNDARY + 1], 0x8b);
    assert_eq!(
        expected.locations,
        [ExpectedLocation {
            code_point: 0x200B,
            byte_offset: 65_535,
            scalar_offset: 65_535,
        }]
    );
}

fn oracle_locations(input: &str, ranges: &[CodePointRange]) -> Vec<ExpectedLocation> {
    input
        .char_indices()
        .enumerate()
        .filter_map(|(scalar_offset, (byte_offset, value))| {
            if ranges
                .iter()
                .copied()
                .any(|range| range.contains(u32::from(value)))
            {
                Some(ExpectedLocation {
                    code_point: u32::from(value),
                    byte_offset: u64::try_from(byte_offset).expect("fixture offset fits u64"),
                    scalar_offset: u64::try_from(scalar_offset).expect("fixture offset fits u64"),
                })
            } else {
                None
            }
        })
        .collect()
}

fn full_pinned_ucd_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/library/unicode/17.0.0/DerivedCoreProperties.txt")
}
