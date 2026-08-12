use std::fmt::Write as _;

use scrub_report::{Evidence, FindingStatus};

pub(crate) const INSPECTION_READ_BOUNDARY: usize = 64 * 1024;
pub(crate) const RETAINED_LOCATION_LIMIT: usize = 256;

const BIDI_CONTROL_PROPERTY: &str = "Bidi_Control";
const PINNED_BIDI_CONTROL_DATA: &str = include_str!("../fixtures/unicode-bidi-control-17.0.0.txt");

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ControlIdentity {
    pub(crate) code_point: u32,
    pub(crate) abbreviation: &'static str,
}

const CONTROL_IDENTITIES: [ControlIdentity; 12] = [
    ControlIdentity {
        code_point: 0x061C,
        abbreviation: "ALM",
    },
    ControlIdentity {
        code_point: 0x200E,
        abbreviation: "LRM",
    },
    ControlIdentity {
        code_point: 0x200F,
        abbreviation: "RLM",
    },
    ControlIdentity {
        code_point: 0x202A,
        abbreviation: "LRE",
    },
    ControlIdentity {
        code_point: 0x202B,
        abbreviation: "RLE",
    },
    ControlIdentity {
        code_point: 0x202C,
        abbreviation: "PDF",
    },
    ControlIdentity {
        code_point: 0x202D,
        abbreviation: "LRO",
    },
    ControlIdentity {
        code_point: 0x202E,
        abbreviation: "RLO",
    },
    ControlIdentity {
        code_point: 0x2066,
        abbreviation: "LRI",
    },
    ControlIdentity {
        code_point: 0x2067,
        abbreviation: "RLI",
    },
    ControlIdentity {
        code_point: 0x2068,
        abbreviation: "FSI",
    },
    ControlIdentity {
        code_point: 0x2069,
        abbreviation: "PDI",
    },
];

pub(crate) fn control_identities() -> &'static [ControlIdentity; 12] {
    &CONTROL_IDENTITIES
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExpectedLocation {
    pub(crate) code_point: u32,
    pub(crate) abbreviation: &'static str,
    pub(crate) byte_offset: u64,
    pub(crate) scalar_offset: u64,
}

impl ExpectedLocation {
    fn new(code_point: u32, byte_offset: u64, scalar_offset: u64) -> Self {
        let abbreviation = CONTROL_IDENTITIES
            .iter()
            .find(|identity| identity.code_point == code_point)
            .expect("fixture locations use a canonical Bidi_Control identity")
            .abbreviation;
        Self {
            code_point,
            abbreviation,
            byte_offset,
            scalar_offset,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ExpectedValidObservation {
    pub(crate) status: FindingStatus,
    pub(crate) total_occurrence_count: u64,
    pub(crate) locations: Vec<ExpectedLocation>,
    pub(crate) locations_truncated: bool,
}

impl ExpectedValidObservation {
    fn new(
        status: FindingStatus,
        total_occurrence_count: u64,
        locations: Vec<ExpectedLocation>,
        locations_truncated: bool,
    ) -> Self {
        Self {
            status,
            total_occurrence_count,
            locations,
            locations_truncated,
        }
    }

    pub(crate) fn report_evidence(&self) -> Vec<Evidence> {
        vec![
            Evidence::new("locations", self.locations_json()),
            Evidence::new("locations_truncated", self.locations_truncated.to_string()),
            Evidence::new(
                "total_occurrence_count",
                self.total_occurrence_count.to_string(),
            ),
        ]
    }

    fn locations_json(&self) -> String {
        let mut value = String::from("[");
        for (index, location) in self.locations.iter().enumerate() {
            if index != 0 {
                value.push(',');
            }
            write!(
                value,
                "{{\"code_point\":\"{}\",\"abbreviation\":\"{}\",\"byte_offset\":{},\"scalar_offset\":{}}}",
                canonical_code_point(location.code_point),
                location.abbreviation,
                location.byte_offset,
                location.scalar_offset
            )
            .expect("writing to a String cannot fail");
        }
        value.push(']');
        value
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum ExpectedObservation {
    Valid(ExpectedValidObservation),
    InvalidUtf8,
}

impl ExpectedObservation {
    pub(crate) const fn status(&self) -> FindingStatus {
        match self {
            Self::Valid(expected) => expected.status,
            Self::InvalidUtf8 => FindingStatus::Invalid,
        }
    }

    pub(crate) fn property_evidence(&self) -> Vec<Evidence> {
        match self {
            Self::Valid(expected) => expected.report_evidence(),
            Self::InvalidUtf8 => vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExpectedArtifactIdentity {
    pub(crate) byte_length: u64,
    pub(crate) sha256: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Fixture {
    pub(crate) name: &'static str,
    pub(crate) input: Vec<u8>,
    pub(crate) expected: ExpectedObservation,
    pub(crate) frozen_artifact_identity: Option<ExpectedArtifactIdentity>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct CodePointRange {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

impl CodePointRange {
    pub(crate) const fn code_point_count(self) -> u32 {
        self.end - self.start + 1
    }

    pub(crate) const fn contains(self, value: u32) -> bool {
        self.start <= value && value <= self.end
    }
}

pub(crate) fn pinned_bidi_control_ranges() -> Vec<CodePointRange> {
    parse_bidi_control_ranges(PINNED_BIDI_CONTROL_DATA)
        .expect("the checked-in Bidi_Control extract is valid")
}

pub(crate) fn parse_bidi_control_ranges(input: &str) -> Result<Vec<CodePointRange>, String> {
    let mut ranges = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }

        let Some((code_points, property)) = data.split_once(';') else {
            if data.contains(BIDI_CONTROL_PROPERTY) {
                return Err(format!("line {} has no property separator", line_index + 1));
            }
            continue;
        };
        if property.trim() != BIDI_CONTROL_PROPERTY {
            continue;
        }

        let code_points = code_points.trim();
        let (start, end) = match code_points.split_once("..") {
            Some((start, end)) => (parse_hex(start, line_index)?, parse_hex(end, line_index)?),
            None => {
                let value = parse_hex(code_points, line_index)?;
                (value, value)
            }
        };
        if start > end {
            return Err(format!("line {} has a descending range", line_index + 1));
        }
        ranges.push(CodePointRange { start, end });
    }
    Ok(ranges)
}

fn parse_hex(value: &str, line_index: usize) -> Result<u32, String> {
    u32::from_str_radix(value.trim(), 16)
        .map_err(|_| format!("line {} has an invalid code point", line_index + 1))
}

pub(crate) fn fixture_corpus() -> Vec<Fixture> {
    vec![
        valid_absent("empty_utf8", ""),
        valid_absent("plain_ascii", "Plain ASCII text 123."),
        valid_absent(
            "benign_non_ascii_without_controls",
            "na\u{ef}ve caf\u{e9} \u{2014} \u{0395}\u{03bb}\u{03bb}\u{03b7}\u{03bd}\u{03b9}\u{03ba}\u{03ac} \u{4e2d}\u{6587} \u{1f600}",
        ),
        valid_absent(
            "ordinary_arabic_without_controls",
            "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}",
        ),
        valid_absent(
            "ordinary_hebrew_without_controls",
            "\u{05e9}\u{05dc}\u{05d5}\u{05dd} \u{05e2}\u{05d5}\u{05dc}\u{05dd}",
        ),
        valid_absent(
            "mixed_rtl_ltr_digits_without_controls",
            "abc \u{05e9}\u{05dc}\u{05d5}\u{05dd} 123 \u{0645}\u{0631}\u{062d}\u{0628}\u{0627} xyz",
        ),
        valid_absent("dicp_non_bidi_zwj", "a\u{200d}b"),
        valid_present(
            "identity_alm_at_beginning",
            "\u{061c}alpha",
            vec![ExpectedLocation::new(0x061C, 0, 0)],
        ),
        valid_present(
            "identity_lrm_in_middle",
            "a\u{200e}b",
            vec![ExpectedLocation::new(0x200E, 1, 1)],
        ),
        valid_present(
            "identity_rlm_at_end",
            "ab\u{200f}",
            vec![ExpectedLocation::new(0x200F, 2, 2)],
        ),
        valid_present(
            "identity_lre_repeated",
            "x\u{202a}\u{202a}",
            vec![
                ExpectedLocation::new(0x202A, 1, 1),
                ExpectedLocation::new(0x202A, 4, 2),
            ],
        ),
        valid_present(
            "identity_rle_after_multibyte_prefix",
            "\u{00e9}\u{754c}\u{202b}",
            vec![ExpectedLocation::new(0x202B, 5, 2)],
        ),
        valid_present(
            "identity_pdf_after_supplementary_scalar",
            "\u{1f600}\u{202c}",
            vec![ExpectedLocation::new(0x202C, 4, 1)],
        ),
        valid_present(
            "identity_lro_after_ascii_prefix",
            "abc\u{202d}",
            vec![ExpectedLocation::new(0x202D, 3, 3)],
        ),
        valid_present(
            "identity_rlo_only",
            "\u{202e}",
            vec![ExpectedLocation::new(0x202E, 0, 0)],
        ),
        valid_present(
            "identity_lri_only",
            "\u{2066}",
            vec![ExpectedLocation::new(0x2066, 0, 0)],
        ),
        valid_present(
            "identity_rli_only",
            "\u{2067}",
            vec![ExpectedLocation::new(0x2067, 0, 0)],
        ),
        valid_present(
            "identity_fsi_only",
            "\u{2068}",
            vec![ExpectedLocation::new(0x2068, 0, 0)],
        ),
        valid_present(
            "identity_pdi_only",
            "\u{2069}",
            vec![ExpectedLocation::new(0x2069, 0, 0)],
        ),
        structure_fixture("structure_lre_then_pdf", 0x202A, 0x202C),
        structure_fixture("structure_rle_then_pdf", 0x202B, 0x202C),
        structure_fixture("structure_lri_then_pdi", 0x2066, 0x2069),
        structure_fixture("structure_rli_then_pdi", 0x2067, 0x2069),
        structure_fixture("structure_fsi_then_pdi", 0x2068, 0x2069),
        nested_structure_fixture(),
        all_control_identities_fixture(),
        repeated_occurrences(256),
        repeated_occurrences(257),
        valid_boundary_fixture(),
        invalid(
            "lone_continuation_byte",
            vec![0x80],
            1,
            "76be8b528d0075f7aae98d6fa57a6d3c83ae480a8469e668d7b0af968995ac71",
        ),
        invalid(
            "truncated_multibyte_sequence_at_eof",
            vec![0xe2, 0x80],
            2,
            "96ea1f62357a87ee51fcc63ac8d01fdcdce2ac660b037b345b02881547e3acb8",
        ),
        invalid(
            "overlong_encoding",
            vec![0xc0, 0xaf],
            2,
            "caf573f0daa6960ecb26f8eddbc4e2059277ad5afc6f72ffd59a0ecead602a22",
        ),
        invalid(
            "utf8_surrogate_encoding",
            vec![0xed, 0xa0, 0x80],
            3,
            "91a681b998555fb475479817b126c94e57e52011fa1842c5d188795a4a05226b",
        ),
        invalid(
            "valid_control_prefix_then_invalid_utf8",
            vec![0xe2, 0x80, 0xae, 0xff],
            4,
            "e956c3e92e2541a8cd5cc9997256fb5494751b88995e6788042ecd3fda1d9fd1",
        ),
        malformed_boundary_fixture(),
    ]
}

fn valid_absent(name: &'static str, input: &str) -> Fixture {
    Fixture {
        name,
        input: input.as_bytes().to_vec(),
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Absent,
            0,
            vec![],
            false,
        )),
        frozen_artifact_identity: None,
    }
}

fn valid_present(name: &'static str, input: &str, locations: Vec<ExpectedLocation>) -> Fixture {
    let total_occurrence_count =
        u64::try_from(locations.len()).expect("fixture location count fits u64");
    Fixture {
        name,
        input: input.as_bytes().to_vec(),
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Present,
            total_occurrence_count,
            locations,
            false,
        )),
        frozen_artifact_identity: None,
    }
}

fn structure_fixture(name: &'static str, opener: u32, closer: u32) -> Fixture {
    let opener = char::from_u32(opener).expect("structure opener is a scalar value");
    let closer = char::from_u32(closer).expect("structure closer is a scalar value");
    let input = format!("{opener}text{closer}");
    valid_present(
        name,
        &input,
        vec![
            ExpectedLocation::new(u32::from(opener), 0, 0),
            ExpectedLocation::new(u32::from(closer), 7, 5),
        ],
    )
}

fn nested_structure_fixture() -> Fixture {
    valid_present(
        "structure_isolate_with_override",
        "\u{2066}\u{202d}x\u{202c}\u{2069}",
        vec![
            ExpectedLocation::new(0x2066, 0, 0),
            ExpectedLocation::new(0x202D, 3, 1),
            ExpectedLocation::new(0x202C, 7, 3),
            ExpectedLocation::new(0x2069, 10, 4),
        ],
    )
}

fn all_control_identities_fixture() -> Fixture {
    valid_present(
        "all_control_identities",
        "\u{061c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}",
        vec![
            ExpectedLocation::new(0x061C, 0, 0),
            ExpectedLocation::new(0x200E, 2, 1),
            ExpectedLocation::new(0x200F, 5, 2),
            ExpectedLocation::new(0x202A, 8, 3),
            ExpectedLocation::new(0x202B, 11, 4),
            ExpectedLocation::new(0x202C, 14, 5),
            ExpectedLocation::new(0x202D, 17, 6),
            ExpectedLocation::new(0x202E, 20, 7),
            ExpectedLocation::new(0x2066, 23, 8),
            ExpectedLocation::new(0x2067, 26, 9),
            ExpectedLocation::new(0x2068, 29, 10),
            ExpectedLocation::new(0x2069, 32, 11),
        ],
    )
}

fn repeated_occurrences(count: usize) -> Fixture {
    let retained = count.min(RETAINED_LOCATION_LIMIT);
    let locations = (0..retained)
        .map(|index| {
            let index = u64::try_from(index).expect("fixture index fits u64");
            ExpectedLocation::new(0x202E, index * 3, index)
        })
        .collect();
    Fixture {
        name: if count == RETAINED_LOCATION_LIMIT {
            "exactly_256_bidi_controls"
        } else {
            "exactly_257_bidi_controls"
        },
        input: "\u{202e}".repeat(count).into_bytes(),
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Present,
            u64::try_from(count).expect("fixture count fits u64"),
            locations,
            count > RETAINED_LOCATION_LIMIT,
        )),
        frozen_artifact_identity: None,
    }
}

fn valid_boundary_fixture() -> Fixture {
    let byte_offset = INSPECTION_READ_BOUNDARY - 1;
    let mut input = vec![b'a'; byte_offset];
    input.extend_from_slice("\u{202e}".as_bytes());
    input.push(b'z');
    Fixture {
        name: "valid_control_crosses_read_boundary",
        input,
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Present,
            1,
            vec![ExpectedLocation::new(
                0x202E,
                u64::try_from(byte_offset).expect("boundary offset fits u64"),
                u64::try_from(byte_offset).expect("boundary offset fits u64"),
            )],
            false,
        )),
        frozen_artifact_identity: Some(ExpectedArtifactIdentity {
            byte_length: 65_539,
            sha256: "324277bc492569075d693fecfc01fc21f1c8d84beca5c79f3da98aaa5ed27131",
        }),
    }
}

fn invalid(name: &'static str, input: Vec<u8>, byte_length: u64, sha256: &'static str) -> Fixture {
    Fixture {
        name,
        input,
        expected: ExpectedObservation::InvalidUtf8,
        frozen_artifact_identity: Some(ExpectedArtifactIdentity {
            byte_length,
            sha256,
        }),
    }
}

fn malformed_boundary_fixture() -> Fixture {
    let mut input = "\u{202e}".as_bytes().to_vec();
    input.extend(std::iter::repeat_n(
        b'a',
        INSPECTION_READ_BOUNDARY - 1 - input.len(),
    ));
    input.extend_from_slice(&[0xe2, 0x28, 0xa1]);
    input.extend(std::iter::repeat_n(b'z', INSPECTION_READ_BOUNDARY));
    invalid(
        "malformed_utf8_crosses_read_boundary",
        input,
        131_074,
        "a8eefc7237d54ce856f771ccc02df988b09f1ede84eb3fc6e0fb64a5018a8be0",
    )
}

fn canonical_code_point(code_point: u32) -> String {
    if code_point <= 0xFFFF {
        format!("U+{code_point:04X}")
    } else {
        format!("U+{code_point:X}")
    }
}
