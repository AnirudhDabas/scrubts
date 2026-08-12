use std::fmt::Write as _;

use scrub_report::{Evidence, FindingStatus};

pub(crate) const INSPECTION_READ_BOUNDARY: usize = 64 * 1024;
pub(crate) const RETAINED_LOCATION_LIMIT: usize = 256;

const DICP_PROPERTY: &str = "Default_Ignorable_Code_Point";
const PINNED_DICP_DATA: &str = include_str!("../fixtures/unicode-default-ignorable-17.0.0.txt");

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExpectedLocation {
    pub(crate) code_point: u32,
    pub(crate) byte_offset: u64,
    pub(crate) scalar_offset: u64,
}

impl ExpectedLocation {
    const fn new(code_point: u32, byte_offset: u64, scalar_offset: u64) -> Self {
        Self {
            code_point,
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
                "{{\"code_point\":\"{}\",\"byte_offset\":{},\"scalar_offset\":{}}}",
                canonical_code_point(location.code_point),
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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Fixture {
    pub(crate) name: &'static str,
    pub(crate) input: Vec<u8>,
    pub(crate) expected: ExpectedObservation,
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

pub(crate) fn pinned_dicp_ranges() -> Vec<CodePointRange> {
    parse_dicp_ranges(PINNED_DICP_DATA).expect("the checked-in DICP extract is valid")
}

pub(crate) fn parse_dicp_ranges(input: &str) -> Result<Vec<CodePointRange>, String> {
    let mut ranges = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }

        let Some((code_points, property)) = data.split_once(';') else {
            if data.contains(DICP_PROPERTY) {
                return Err(format!("line {} has no property separator", line_index + 1));
            }
            continue;
        };
        if property.trim() != DICP_PROPERTY {
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
        valid_absent("ordinary_benign_non_ascii", "naïve café — Ελληνικά 中文 😀"),
        valid_absent("non_dicp_combining_marks", "a\u{0301} o\u{20dd}"),
        valid_present(
            "zero_width_space_at_beginning",
            "\u{200b}alpha",
            vec![ExpectedLocation::new(0x200B, 0, 0)],
        ),
        valid_present(
            "zero_width_non_joiner_in_middle",
            "a\u{200c}b",
            vec![ExpectedLocation::new(0x200C, 1, 1)],
        ),
        valid_present(
            "zero_width_joiner_at_end",
            "a\u{200d}",
            vec![ExpectedLocation::new(0x200D, 1, 1)],
        ),
        valid_present(
            "zero_width_no_break_space_feff",
            "\u{feff}",
            vec![ExpectedLocation::new(0xFEFF, 0, 0)],
        ),
        valid_present(
            "emoji_heart_with_variation_selector_16",
            "\u{2764}\u{fe0f}",
            vec![ExpectedLocation::new(0xFE0F, 3, 1)],
        ),
        valid_present(
            "supplementary_variation_selector_17",
            "\u{e0100}",
            vec![ExpectedLocation::new(0xE0100, 0, 0)],
        ),
        valid_present(
            "repeated_zero_width_spaces",
            "a\u{200b}\u{200b}b",
            vec![
                ExpectedLocation::new(0x200B, 1, 1),
                ExpectedLocation::new(0x200B, 4, 2),
            ],
        ),
        valid_present(
            "ascii_before_match",
            "abc\u{200b}",
            vec![ExpectedLocation::new(0x200B, 3, 3)],
        ),
        valid_present(
            "multibyte_scalars_before_match",
            "é界\u{200b}",
            vec![ExpectedLocation::new(0x200B, 5, 2)],
        ),
        valid_present(
            "supplementary_scalar_before_supplementary_match",
            "😀\u{e0100}",
            vec![ExpectedLocation::new(0xE0100, 4, 1)],
        ),
        repeated_occurrences(256),
        repeated_occurrences(257),
        boundary_spanning_fixture(),
        invalid("invalid_leading_byte", vec![0xff]),
        invalid("unexpected_continuation_byte", vec![0x80]),
        invalid("overlong_encoding", vec![0xc0, 0xaf]),
        invalid("utf8_surrogate_encoding", vec![0xed, 0xa0, 0x80]),
        invalid(
            "code_point_above_unicode_maximum",
            vec![0xf4, 0x90, 0x80, 0x80],
        ),
        invalid("truncated_multibyte_sequence_at_eof", vec![0xe2, 0x80]),
        invalid(
            "valid_dicp_prefix_then_invalid_utf8",
            vec![0xe2, 0x80, 0x8b, 0xff],
        ),
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
    }
}

fn repeated_occurrences(count: usize) -> Fixture {
    let retained = count.min(RETAINED_LOCATION_LIMIT);
    let locations = (0..retained)
        .map(|index| {
            let index = u64::try_from(index).expect("fixture index fits u64");
            ExpectedLocation::new(0x200B, index * 3, index)
        })
        .collect();
    Fixture {
        name: if count == RETAINED_LOCATION_LIMIT {
            "exactly_256_occurrences"
        } else {
            "257_occurrences"
        },
        input: "\u{200b}".repeat(count).into_bytes(),
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Present,
            u64::try_from(count).expect("fixture count fits u64"),
            locations,
            count > RETAINED_LOCATION_LIMIT,
        )),
    }
}

fn boundary_spanning_fixture() -> Fixture {
    let byte_offset = INSPECTION_READ_BOUNDARY - 1;
    let mut input = vec![b'a'; byte_offset];
    input.extend_from_slice("\u{200b}".as_bytes());
    input.push(b'z');
    Fixture {
        name: "dicp_spans_64_kib_read_boundary",
        input,
        expected: ExpectedObservation::Valid(ExpectedValidObservation::new(
            FindingStatus::Present,
            1,
            vec![ExpectedLocation::new(
                0x200B,
                u64::try_from(byte_offset).expect("boundary offset fits u64"),
                u64::try_from(byte_offset).expect("boundary offset fits u64"),
            )],
            false,
        )),
    }
}

fn invalid(name: &'static str, input: Vec<u8>) -> Fixture {
    Fixture {
        name,
        input,
        expected: ExpectedObservation::InvalidUtf8,
    }
}

fn canonical_code_point(code_point: u32) -> String {
    if code_point <= 0xFFFF {
        format!("U+{code_point:04X}")
    } else {
        format!("U+{code_point:X}")
    }
}
