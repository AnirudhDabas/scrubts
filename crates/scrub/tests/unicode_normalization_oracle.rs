#[path = "support/unicode_normalization.rs"]
mod unicode_normalization;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use scrub_report::Sha256Digest;
use sha2::{Digest, Sha256};
use unicode_normalization::{
    DERIVED_AGE_BYTES, NORMALIZATION_TEST_BYTES, NormalizationForm, SURROGATE_END, SURROGATE_START,
    UNICODE_MAX, assigned_code_points, assigned_complement, identity_expectations,
    is_unicode_scalar, parse_derived_age, parse_normalization_test, part1_source_code_points,
    unassigned_scalar_values, unicode_scalar_values,
};

const NORMALIZATION_TEST_BYTE_LENGTH: usize = 2_827_429;
const NORMALIZATION_TEST_SHA256: &str =
    "5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db";
const DERIVED_AGE_BYTE_LENGTH: usize = 138_286;
const DERIVED_AGE_SHA256: &str = "f8ecdf768bdc210f201abd271d9bc587825618a86a7046a8146cc816393f1998";
const UNICODE_LICENSE_BYTE_LENGTH: usize = 1_995;
const UNICODE_LICENSE_SHA256: &str =
    "e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96";

#[test]
fn committed_official_fixtures_and_license_have_pinned_raw_identities() {
    assert_eq!(
        NORMALIZATION_TEST_BYTES.len(),
        NORMALIZATION_TEST_BYTE_LENGTH
    );
    assert_sha256(NORMALIZATION_TEST_BYTES, NORMALIZATION_TEST_SHA256);
    assert_eq!(DERIVED_AGE_BYTES.len(), DERIVED_AGE_BYTE_LENGTH);
    assert_sha256(DERIVED_AGE_BYTES, DERIVED_AGE_SHA256);

    let license = include_bytes!("fixtures/UNICODE-LICENSE.txt");
    assert_eq!(license.len(), UNICODE_LICENSE_BYTE_LENGTH);
    assert_sha256(license, UNICODE_LICENSE_SHA256);
}

#[test]
fn fixture_headers_pin_unicode_17_identity_and_versions() {
    let normalization = std::str::from_utf8(NORMALIZATION_TEST_BYTES).expect("fixture is UTF-8");
    assert!(
        normalization
            .starts_with("# NormalizationTest-17.0.0.txt\n# Date: 2025-06-30, 06:16:16 GMT\n")
    );
    assert!(normalization.contains("#      source; NFC; NFD; NFKC; NFKD\n"));
    assert!(normalization.contains("#      X == toNFC(X) == toNFD(X) == toNFKC(X) == toNFKD(X)\n"));

    let age = std::str::from_utf8(DERIVED_AGE_BYTES).expect("fixture is UTF-8");
    assert!(age.starts_with("# DerivedAge-17.0.0.txt\n# Date: 2025-07-30, 23:54:38 GMT\n"));
    assert!(age.contains("# @missing: 0000..10FFFF; Unassigned\n"));
}

#[test]
fn all_normalization_records_parse_with_exact_part_structure() {
    let records = parse_normalization_test(NORMALIZATION_TEST_BYTES)
        .expect("the complete official normalization fixture parses");
    let mut part_counts = [0_usize; 6];
    for record in &records {
        part_counts[usize::from(record.part)] += 1;
        assert!(
            record.columns.iter().all(|sequence| !sequence.is_empty()),
            "each of the five semantic sequences is nonempty"
        );
        assert!(
            record
                .columns
                .iter()
                .flatten()
                .all(|code_point| *code_point <= UNICODE_MAX)
        );
    }

    assert_eq!(records.len(), 20_034);
    assert_eq!(part_counts, [45, 17_086, 1_936, 194, 735, 38]);
}

#[test]
fn normalization_parser_rejects_malformed_or_unrecognized_records() {
    let valid = b"@Part0\n0041;0041;0041;0041;0041;\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n";
    assert_eq!(
        parse_normalization_test(valid)
            .expect("minimal syntax parses")
            .len(),
        1
    );

    for prohibited_line_ending in [
        b"@Part0\r\n0041;0041;0041;0041;0041;\r\n@Part1\r\n@Part2\r\n@Part3\r\n@Part4\r\n@Part5\r\n"
            .as_slice(),
        b"@Part0\r0041;0041;0041;0041;0041;\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
    ] {
        let error = parse_normalization_test(prohibited_line_ending)
            .expect_err("carriage returns are prohibited before line parsing");
        assert!(error.to_string().contains("carriage return is not permitted"));
    }

    for malformed in [
        b"@Part0\ngarbage\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n".as_slice(),
        b"@Part0\n0041;0041;0041;0041;\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
        b"@Part0\n0041;0041;0041;0041;0041\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
        b"@Part0\n0041;0041;0041;0041;00e9;\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
        b"@Part0\n0041;0041;0041;0041;110000;\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
        b"@Part0\n@Unknown\n@Part1\n@Part2\n@Part3\n@Part4\n@Part5\n",
        b"@Part1\n@Part0\n@Part2\n@Part3\n@Part4\n@Part5\n",
    ] {
        assert!(parse_normalization_test(malformed).is_err());
    }
}

#[test]
fn literal_header_relationships_expand_to_all_400680_cases() {
    let records = parse_normalization_test(NORMALIZATION_TEST_BYTES)
        .expect("the complete official normalization fixture parses");
    let count = records
        .iter()
        .flat_map(|record| record.expected_transformations())
        .count();
    assert_eq!(count, 400_680);

    let record = records
        .iter()
        .find(|record| record.columns[0] == [0x00C0])
        .expect("the U+00C0 Part 1 row exists");
    assert_eq!(record.columns[1], [0x00C0]);
    assert_eq!(record.columns[2], [0x0041, 0x0300]);
    assert_eq!(record.columns[3], [0x00C0]);
    assert_eq!(record.columns[4], [0x0041, 0x0300]);

    let actual: Vec<_> = record
        .expected_transformations()
        .map(|case| {
            (
                case.form,
                case.input_column,
                case.expected_column,
                case.input.to_vec(),
                case.expected.to_vec(),
            )
        })
        .collect();
    assert_eq!(actual.len(), 20);
    assert_eq!(
        actual[0],
        (NormalizationForm::Nfc, 0, 1, vec![0x00C0], vec![0x00C0])
    );
    assert_eq!(
        actual[7],
        (
            NormalizationForm::Nfd,
            2,
            2,
            vec![0x0041, 0x0300],
            vec![0x0041, 0x0300],
        )
    );
    assert_eq!(
        actual[14],
        (
            NormalizationForm::Nfkc,
            4,
            3,
            vec![0x0041, 0x0300],
            vec![0x00C0],
        )
    );
    assert_eq!(
        actual[19],
        (
            NormalizationForm::Nfkd,
            4,
            4,
            vec![0x0041, 0x0300],
            vec![0x0041, 0x0300],
        )
    );

    let marker_record = unicode_normalization::NormalizationRecord {
        part: 0,
        columns: [vec![1], vec![2], vec![3], vec![4], vec![5]],
    };
    let mapping: Vec<_> = marker_record
        .expected_transformations()
        .map(|case| (case.form, case.input_column, case.expected_column))
        .collect();
    assert_eq!(
        mapping,
        [
            (NormalizationForm::Nfc, 0, 1),
            (NormalizationForm::Nfc, 1, 1),
            (NormalizationForm::Nfc, 2, 1),
            (NormalizationForm::Nfc, 3, 3),
            (NormalizationForm::Nfc, 4, 3),
            (NormalizationForm::Nfd, 0, 2),
            (NormalizationForm::Nfd, 1, 2),
            (NormalizationForm::Nfd, 2, 2),
            (NormalizationForm::Nfd, 3, 4),
            (NormalizationForm::Nfd, 4, 4),
            (NormalizationForm::Nfkc, 0, 3),
            (NormalizationForm::Nfkc, 1, 3),
            (NormalizationForm::Nfkc, 2, 3),
            (NormalizationForm::Nfkc, 3, 3),
            (NormalizationForm::Nfkc, 4, 3),
            (NormalizationForm::Nfkd, 0, 4),
            (NormalizationForm::Nfkd, 1, 4),
            (NormalizationForm::Nfkd, 2, 4),
            (NormalizationForm::Nfkd, 3, 4),
            (NormalizationForm::Nfkd, 4, 4),
        ]
    );
}

#[test]
fn derived_age_assignment_and_part1_complement_have_frozen_extents() {
    let age_records =
        parse_derived_age(DERIVED_AGE_BYTES).expect("the complete official Age fixture parses");
    assert_eq!(age_records.len(), 1_815);
    assert!(age_records.iter().all(|record| !record.version.is_empty()));

    let assigned = assigned_code_points(&age_records).expect("Age ranges do not overlap");
    let normalization_records = parse_normalization_test(NORMALIZATION_TEST_BYTES)
        .expect("the complete official normalization fixture parses");
    let part1_sources = part1_source_code_points(&normalization_records);
    assert!(
        normalization_records
            .iter()
            .filter(|record| record.part == 1)
            .all(|record| record.columns[0].len() == 1)
    );
    let complement = assigned_complement(&assigned, &part1_sources);
    let surrogate_complement = complement.range(SURROGATE_START..=SURROGATE_END).count();
    let assigned_scalar_complement = complement
        .iter()
        .filter(|code_point| is_unicode_scalar(**code_point))
        .count();

    assert_eq!(assigned.len(), 299_448);
    assert_eq!(part1_sources.len(), 17_086);
    assert!(part1_sources.is_subset(&assigned));
    assert_eq!(complement.len(), 282_362);
    assert_eq!(surrogate_complement, 2_048);
    assert_eq!(assigned_scalar_complement, 280_314);

    let expected_identity_cases = identity_expectations(
        complement
            .iter()
            .copied()
            .filter(|code_point| is_unicode_scalar(*code_point)),
    )
    .count();
    assert_eq!(expected_identity_cases, 1_121_256);
}

#[test]
fn unassigned_scalar_domain_freezes_ordinary_normalization_identity_obligation() {
    let age_records =
        parse_derived_age(DERIVED_AGE_BYTES).expect("the complete official Age fixture parses");
    let assigned = assigned_code_points(&age_records).expect("Age ranges do not overlap");
    let total_scalars = unicode_scalar_values().count();
    let assigned_surrogates = assigned.range(SURROGATE_START..=SURROGATE_END).count();
    let assigned_scalars = assigned
        .iter()
        .filter(|code_point| is_unicode_scalar(**code_point))
        .count();
    let unassigned: BTreeSet<_> = unassigned_scalar_values(&assigned).collect();

    assert_eq!(UNICODE_MAX + 1, 1_114_112);
    assert_eq!(SURROGATE_END - SURROGATE_START + 1, 2_048);
    assert_eq!(total_scalars, 1_112_064);
    assert_eq!(assigned_surrogates, 2_048);
    assert_eq!(assigned_scalars, 297_400);
    assert_eq!(unassigned.len(), 814_664);
    assert!(
        unassigned
            .iter()
            .all(|code_point| is_unicode_scalar(*code_point))
    );
    assert!(unassigned.is_disjoint(&assigned));

    let mut cases = 0_usize;
    let first_scalar_forms: Vec<_> = identity_expectations(unassigned.iter().copied().take(1))
        .map(|expectation| expectation.form)
        .collect();
    assert_eq!(
        first_scalar_forms,
        [
            NormalizationForm::Nfc,
            NormalizationForm::Nfd,
            NormalizationForm::Nfkc,
            NormalizationForm::Nfkd,
        ]
    );
    for expectation in identity_expectations(unassigned.iter().copied()) {
        assert_eq!(expectation.input, expectation.expected);
        assert!(unassigned.contains(&expectation.input));
        cases += 1;
    }
    assert_eq!(cases, 3_258_656);
}

#[test]
fn derived_age_parser_rejects_malformed_ranges_and_overlaps() {
    let boundary_versions = parse_derived_age(b"0041 ; 1.0\n0042 ; 255.255\n")
        .expect("UAX #44 Age component boundaries parse");
    assert_eq!(boundary_versions[0].version, "1.0");
    assert_eq!(boundary_versions[1].version, "255.255");

    for prohibited_line_ending in [b"0041 ; 1.1\r\n".as_slice(), b"0041 ; 1.1\r0042 ; 2.0\n"] {
        let error = parse_derived_age(prohibited_line_ending)
            .expect_err("carriage returns are prohibited before line parsing");
        assert!(
            error
                .to_string()
                .contains("carriage return is not permitted")
        );
    }

    assert!(parse_derived_age(b"0041..0040 ; 1.1\n").is_err());
    assert!(parse_derived_age(b"0041 ; one\n").is_err());
    assert!(parse_derived_age(b"00e9 ; 1.1\n").is_err());
    assert!(parse_derived_age(b"110000 ; 1.1\n").is_err());
    for invalid_version in [
        "0.0",
        "256.0",
        "1.256",
        "999999999999999999999999999999999999999999999999999999999999.0",
        "1.999999999999999999999999999999999999999999999999999999999999",
    ] {
        let input = format!("0041 ; {invalid_version}\n");
        assert!(
            parse_derived_age(input.as_bytes()).is_err(),
            "invalid Age version {invalid_version:?} must be rejected"
        );
    }

    let overlap = parse_derived_age(b"0041..0042 ; 1.1\n0042 ; 2.0\n")
        .expect("individual overlap records parse");
    assert!(assigned_code_points(&overlap).is_err());
}

#[test]
fn ignored_local_sources_match_committed_fixtures_when_present() {
    for (relative_source, committed) in [
        ("NormalizationTest.txt", NORMALIZATION_TEST_BYTES),
        ("DerivedAge.txt", DERIVED_AGE_BYTES),
        (
            "UNICODE-LICENSE.txt",
            include_bytes!("fixtures/UNICODE-LICENSE.txt").as_slice(),
        ),
    ] {
        let path = local_unicode_source_path(relative_source);
        if !path.is_file() {
            eprintln!(
                "optional source parity input is absent; committed fixture tests remain mandatory: {}",
                path.display()
            );
            continue;
        }
        let source = fs::read(&path).expect("local research source can be read");
        assert_eq!(source, committed, "{} parity", path.display());
    }
}

fn assert_sha256(bytes: &[u8], expected: &str) {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    assert_eq!(Sha256Digest::from_bytes(digest).as_str(), expected);
}

fn local_unicode_source_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/library/unicode/17.0.0")
        .join(name)
}
