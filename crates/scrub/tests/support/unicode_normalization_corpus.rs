use scrub_report::{Evidence, FindingStatus, Sha256Digest};
use sha2::{Digest, Sha256};

pub(crate) const INSPECTION_READ_BOUNDARY: usize = 64 * 1024;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum ExpectedFinding {
    Absent,
    Present {
        normalized: String,
        first_difference: String,
    },
}

impl ExpectedFinding {
    pub(crate) const fn status(&self) -> FindingStatus {
        match self {
            Self::Absent => FindingStatus::Absent,
            Self::Present { .. } => FindingStatus::Present,
        }
    }

    pub(crate) fn evidence(&self) -> Vec<Evidence> {
        match self {
            Self::Absent => vec![],
            Self::Present {
                normalized,
                first_difference,
            } => {
                let digest: [u8; 32] = Sha256::digest(normalized.as_bytes()).into();
                vec![
                    Evidence::new("first_difference", first_difference),
                    Evidence::new("normalized_byte_length", normalized.len().to_string()),
                    Evidence::new(
                        "normalized_scalar_count",
                        normalized.chars().count().to_string(),
                    ),
                    Evidence::new(
                        "normalized_sha256",
                        Sha256Digest::from_bytes(digest).to_string(),
                    ),
                ]
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Fixture {
    pub(crate) name: &'static str,
    pub(crate) input: Vec<u8>,
    pub(crate) nfc: ExpectedFinding,
    pub(crate) nfkc: ExpectedFinding,
}

pub(crate) fn fixture_corpus() -> Vec<Fixture> {
    vec![
        both_absent("empty_valid_utf8", ""),
        both_absent("plain_ascii", "Plain ASCII text 123."),
        both_absent(
            "already_normalized_multilingual",
            "caf\u{e9} \u{0395}\u{03bb}\u{03bb}\u{03b7}\u{03bd}\u{03b9}\u{03ba}\u{03ac} \u{4e2d}\u{6587} \u{1f600}",
        ),
        both_absent("canonical_precomposed", "\u{e9}"),
        both_present(
            "canonical_decomposed",
            "e\u{301}",
            "\u{e9}",
            diff(
                0,
                0,
                "false,[\"U+0065\",\"U+0301\"],false",
                "false,[\"U+00E9\"],false",
            ),
        ),
        both_present(
            "canonical_combining_class_reordering",
            "\u{301}\u{323}",
            "\u{323}\u{301}",
            diff(
                0,
                0,
                "false,[\"U+0301\",\"U+0323\"],false",
                "false,[\"U+0323\",\"U+0301\"],false",
            ),
        ),
        both_absent("blocked_composition", "A\u{342}\u{30a}"),
        both_present(
            "algorithmic_hangul_composition",
            "\u{1100}\u{1161}\u{11a8}",
            "\u{ac01}",
            diff(
                0,
                0,
                "false,[\"U+1100\",\"U+1161\",\"U+11A8\"],false",
                "false,[\"U+AC01\"],false",
            ),
        ),
        nfc_absent_nfkc_present(
            "compatibility_ligature",
            "\u{fb03}",
            "ffi",
            diff(
                0,
                0,
                "false,[\"U+FB03\"],false",
                "false,[\"U+0066\",\"U+0066\",\"U+0069\"],false",
            ),
        ),
        nfc_absent_nfkc_present(
            "fullwidth_compatibility_mapping",
            "\u{ff21}",
            "A",
            diff(0, 0, "false,[\"U+FF21\"],false", "false,[\"U+0041\"],false"),
        ),
        nfc_absent_nfkc_present(
            "superscript_compatibility_mapping",
            "\u{b2}",
            "2",
            diff(0, 0, "false,[\"U+00B2\"],false", "false,[\"U+0032\"],false"),
        ),
        nfc_absent_nfkc_present(
            "circled_compatibility_mapping",
            "\u{2460}",
            "1",
            diff(0, 0, "false,[\"U+2460\"],false", "false,[\"U+0031\"],false"),
        ),
        both_absent("variation_selector_emoji", "\u{2764}\u{fe0f} \u{1f600}"),
        both_present(
            "divergence_after_multibyte_bmp_prefix",
            "\u{754c}e\u{301}",
            "\u{754c}\u{e9}",
            diff(
                1,
                3,
                "false,[\"U+0065\",\"U+0301\"],false",
                "false,[\"U+00E9\"],false",
            ),
        ),
        both_present(
            "divergence_after_supplementary_prefix",
            "\u{1f600}e\u{301}",
            "\u{1f600}\u{e9}",
            diff(
                1,
                4,
                "false,[\"U+0065\",\"U+0301\"],false",
                "false,[\"U+00E9\"],false",
            ),
        ),
        both_present(
            "exactly_eight_first_difference_scalars",
            "e\u{301}abcdfg",
            "\u{e9}abcdfg",
            diff(
                0,
                0,
                "false,[\"U+0065\",\"U+0301\",\"U+0061\",\"U+0062\",\"U+0063\",\"U+0064\",\"U+0066\",\"U+0067\"],false",
                "false,[\"U+00E9\",\"U+0061\",\"U+0062\",\"U+0063\",\"U+0064\",\"U+0066\",\"U+0067\"],false",
            ),
        ),
        both_present(
            "nine_plus_first_difference_scalars",
            "e\u{301}abcdfgh",
            "\u{e9}abcdfgh",
            diff(
                0,
                0,
                "false,[\"U+0065\",\"U+0301\",\"U+0061\",\"U+0062\",\"U+0063\",\"U+0064\",\"U+0066\",\"U+0067\"],true",
                "false,[\"U+00E9\",\"U+0061\",\"U+0062\",\"U+0063\",\"U+0064\",\"U+0066\",\"U+0067\",\"U+0068\"],false",
            ),
        ),
        nfc_absent_nfkc_present(
            "compatibility_decomposition_then_composition_contraction",
            "\u{ff76}\u{ff9e}",
            "\u{30ac}",
            diff(
                0,
                0,
                "false,[\"U+FF76\",\"U+FF9E\"],false",
                "false,[\"U+30AC\"],false",
            ),
        ),
        both_present(
            "back_to_back_sensitive_sequences",
            "e\u{301}o\u{308}",
            "\u{e9}\u{f6}",
            diff(
                0,
                0,
                "false,[\"U+0065\",\"U+0301\",\"U+006F\",\"U+0308\"],false",
                "false,[\"U+00E9\",\"U+00F6\"],false",
            ),
        ),
        valid_multibyte_scalar_crosses_real_boundary(),
        sensitive_sequence_crosses_real_boundary(),
        long_nonstarter_run(),
    ]
}

fn both_absent(name: &'static str, input: &str) -> Fixture {
    Fixture {
        name,
        input: input.as_bytes().to_vec(),
        nfc: ExpectedFinding::Absent,
        nfkc: ExpectedFinding::Absent,
    }
}

fn both_present(
    name: &'static str,
    input: &str,
    normalized: &str,
    first_difference: String,
) -> Fixture {
    Fixture {
        name,
        input: input.as_bytes().to_vec(),
        nfc: ExpectedFinding::Present {
            normalized: normalized.to_owned(),
            first_difference: first_difference.clone(),
        },
        nfkc: ExpectedFinding::Present {
            normalized: normalized.to_owned(),
            first_difference,
        },
    }
}

fn nfc_absent_nfkc_present(
    name: &'static str,
    input: &str,
    normalized_nfkc: &str,
    first_difference: String,
) -> Fixture {
    Fixture {
        name,
        input: input.as_bytes().to_vec(),
        nfc: ExpectedFinding::Absent,
        nfkc: ExpectedFinding::Present {
            normalized: normalized_nfkc.to_owned(),
            first_difference,
        },
    }
}

fn valid_multibyte_scalar_crosses_real_boundary() -> Fixture {
    let mut input = vec![b'a'; INSPECTION_READ_BOUNDARY - 1];
    input.extend_from_slice("\u{e9}".as_bytes());
    Fixture {
        name: "valid_multibyte_scalar_crosses_real_65536_boundary",
        input,
        nfc: ExpectedFinding::Absent,
        nfkc: ExpectedFinding::Absent,
    }
}

fn sensitive_sequence_crosses_real_boundary() -> Fixture {
    let scalar_index = INSPECTION_READ_BOUNDARY - 1;
    let mut input = vec![b'a'; scalar_index];
    input.extend_from_slice("e\u{301}".as_bytes());
    let mut normalized = "a".repeat(scalar_index);
    normalized.push('\u{e9}');
    let first_difference = diff(
        u64::try_from(scalar_index).expect("boundary fits u64"),
        u64::try_from(scalar_index).expect("boundary fits u64"),
        "false,[\"U+0065\",\"U+0301\"],false",
        "false,[\"U+00E9\"],false",
    );
    Fixture {
        name: "normalization_sequence_crosses_real_65536_boundary",
        input,
        nfc: ExpectedFinding::Present {
            normalized: normalized.clone(),
            first_difference: first_difference.clone(),
        },
        nfkc: ExpectedFinding::Present {
            normalized,
            first_difference,
        },
    }
}

fn long_nonstarter_run() -> Fixture {
    const NONSTARTERS: usize = 4_096;
    let mut input = String::from("a");
    input.extend(std::iter::repeat_n('\u{315}', NONSTARTERS));
    input.push('\u{300}');
    let mut normalized = String::from("\u{e0}");
    normalized.extend(std::iter::repeat_n('\u{315}', NONSTARTERS));
    let first_difference = diff(
        0,
        0,
        "false,[\"U+0061\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\"],true",
        "false,[\"U+00E0\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\",\"U+0315\"],true",
    );
    both_present(
        "long_nonstarter_run_without_stream_safe_cgj",
        &input,
        &normalized,
        first_difference,
    )
}

fn diff(
    scalar_index: u64,
    original_byte_offset: u64,
    original_window: &str,
    normalized_window: &str,
) -> String {
    format!(
        "{{\"scalar_index\":{scalar_index},\"original_byte_offset\":{original_byte_offset},\"original\":{{\"at_end\":{}}},\"normalized\":{{\"at_end\":{}}}}}",
        window_fields(original_window),
        window_fields(normalized_window)
    )
}

fn window_fields(frozen: &str) -> String {
    let mut fields = frozen.splitn(3, ',');
    let at_end = fields.next().expect("frozen at_end exists");
    let scalars_and_truncated = fields.collect::<Vec<_>>().join(",");
    let split = scalars_and_truncated
        .rfind(',')
        .expect("frozen truncated exists");
    let (scalars, truncated) = scalars_and_truncated.split_at(split);
    format!(
        "{at_end},\"scalars\":{scalars},\"truncated\":{}",
        &truncated[1..]
    )
}
