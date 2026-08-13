#[path = "support/unicode_normalization.rs"]
mod oracle;
#[path = "../src/unicode_normalization.rs"]
mod production;

use std::time::Instant;

use oracle::{
    DERIVED_AGE_BYTES, NORMALIZATION_TEST_BYTES, NormalizationForm, assigned_code_points,
    assigned_complement, identity_expectations, is_unicode_scalar, parse_derived_age,
    parse_normalization_test, part1_source_code_points, unassigned_scalar_values,
};
use production::{ConformanceForm, normalization_matches, normalize_code_points};

#[test]
fn all_400680_official_transformations_match_the_independent_oracle() {
    let started = Instant::now();
    let records = parse_normalization_test(NORMALIZATION_TEST_BYTES)
        .expect("the complete official normalization fixture parses");
    let mut comparisons = 0_u64;

    for (row_index, record) in records.iter().enumerate() {
        for case in record.expected_transformations() {
            assert_case(
                form(case.form),
                case.input,
                case.expected,
                &format!(
                    "official row {row_index}, input c{}, expected c{}",
                    case.input_column + 1,
                    case.expected_column + 1
                ),
            );
            comparisons += 1;
        }
    }

    assert_eq!(comparisons, 400_680);
    eprintln!(
        "normalization conformance: official_transformations={comparisons} elapsed={:?}",
        started.elapsed()
    );
}

#[test]
fn all_1121256_assigned_scalar_identity_cases_match_the_independent_oracle() {
    let started = Instant::now();
    let normalization_records = parse_normalization_test(NORMALIZATION_TEST_BYTES)
        .expect("the complete official normalization fixture parses");
    let age_records =
        parse_derived_age(DERIVED_AGE_BYTES).expect("the complete official Age fixture parses");
    let assigned = assigned_code_points(&age_records).expect("Age ranges do not overlap");
    let part1_sources = part1_source_code_points(&normalization_records);
    let complement = assigned_complement(&assigned, &part1_sources);
    let mut comparisons = 0_u64;

    for case in identity_expectations(
        complement
            .iter()
            .copied()
            .filter(|code_point| is_unicode_scalar(*code_point)),
    ) {
        assert_case(
            form(case.form),
            &[case.input],
            &[case.expected],
            &format!("assigned complement U+{:04X}", case.input),
        );
        comparisons += 1;
    }

    assert_eq!(comparisons, 1_121_256);
    eprintln!(
        "normalization conformance: assigned_scalar_identity={comparisons} elapsed={:?}",
        started.elapsed()
    );
}

#[test]
fn all_3258656_unassigned_scalar_identity_cases_match_the_independent_oracle() {
    let started = Instant::now();
    let age_records =
        parse_derived_age(DERIVED_AGE_BYTES).expect("the complete official Age fixture parses");
    let assigned = assigned_code_points(&age_records).expect("Age ranges do not overlap");
    let mut comparisons = 0_u64;

    for case in identity_expectations(unassigned_scalar_values(&assigned)) {
        assert_case(
            form(case.form),
            &[case.input],
            &[case.expected],
            &format!("unassigned scalar U+{:04X}", case.input),
        );
        comparisons += 1;
    }

    assert_eq!(comparisons, 3_258_656);
    eprintln!(
        "normalization conformance: unassigned_scalar_identity={comparisons} elapsed={:?}",
        started.elapsed()
    );
}

fn form(form: NormalizationForm) -> ConformanceForm {
    match form {
        NormalizationForm::Nfc => ConformanceForm::Nfc,
        NormalizationForm::Nfd => ConformanceForm::Nfd,
        NormalizationForm::Nfkc => ConformanceForm::Nfkc,
        NormalizationForm::Nfkd => ConformanceForm::Nfkd,
    }
}

fn assert_case(form: ConformanceForm, input: &[u32], expected: &[u32], context: &str) {
    if normalization_matches(form, input, expected) {
        return;
    }
    let actual = normalize_code_points(form, input);
    panic!(
        "{context}: {form:?} mismatch\ninput={input:X?}\nexpected={expected:X?}\nactual={actual:X?}"
    );
}
