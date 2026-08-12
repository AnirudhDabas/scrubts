use std::fmt::Write as _;

use scrub_report::{Evidence, Finding, FindingStatus, MechanismIdentity};

pub(crate) const MECHANISM_ID: &str = "unicode.default_ignorable_code_point";
pub(crate) const UNICODE_VERSION: &str = "17.0.0";

const RETAINED_LOCATION_LIMIT: usize = 256;
const VALID_INTERPRETATION_LIMITATION: &str = "Default_Ignorable_Code_Point presence is a neutral Unicode property observation; values may have legitimate formatting, shaping, language, or emoji uses.";
const INVALID_UTF8_LIMITATION: &str = "Default_Ignorable_Code_Point occurrence evidence is unavailable because the complete artifact is not valid UTF-8.";

const DICP_RANGES: [(u32, u32); 27] = [
    (0x00AD, 0x00AD),
    (0x034F, 0x034F),
    (0x061C, 0x061C),
    (0x115F, 0x1160),
    (0x17B4, 0x17B5),
    (0x180B, 0x180D),
    (0x180E, 0x180E),
    (0x180F, 0x180F),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2065, 0x2065),
    (0x2066, 0x206F),
    (0x3164, 0x3164),
    (0xFE00, 0xFE0F),
    (0xFEFF, 0xFEFF),
    (0xFFA0, 0xFFA0),
    (0xFFF0, 0xFFF8),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE0000),
    (0xE0001, 0xE0001),
    (0xE0002, 0xE001F),
    (0xE0020, 0xE007F),
    (0xE0080, 0xE00FF),
    (0xE0100, 0xE01EF),
    (0xE01F0, 0xE0FFF),
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Location {
    code_point: u32,
    byte_offset: u64,
    scalar_offset: u64,
}

pub(crate) struct Inspection {
    incomplete: [u8; 4],
    incomplete_len: usize,
    incomplete_byte_offset: u64,
    scalar_offset: u64,
    total_occurrence_count: u64,
    locations: Vec<Location>,
    invalid_utf8: bool,
}

impl Inspection {
    pub(crate) fn new() -> Self {
        Self {
            incomplete: [0; 4],
            incomplete_len: 0,
            incomplete_byte_offset: 0,
            scalar_offset: 0,
            total_occurrence_count: 0,
            locations: Vec::with_capacity(RETAINED_LOCATION_LIMIT),
            invalid_utf8: false,
        }
    }

    pub(crate) fn inspect_chunk(
        &mut self,
        bytes: &[u8],
        chunk_byte_offset: u64,
    ) -> Result<(), ScanError> {
        if self.invalid_utf8 {
            return Ok(());
        }

        let mut consumed = 0;
        while self.incomplete_len != 0 && consumed < bytes.len() {
            self.incomplete[self.incomplete_len] = bytes[consumed];
            self.incomplete_len += 1;
            consumed += 1;

            let candidate = self.incomplete;
            match std::str::from_utf8(&candidate[..self.incomplete_len]) {
                Ok(text) => {
                    self.inspect_valid_text(text, self.incomplete_byte_offset)?;
                    self.incomplete_len = 0;
                }
                Err(error) if error.error_len().is_none() && self.incomplete_len < 4 => {}
                Err(_) => {
                    self.invalid_utf8 = true;
                    return Ok(());
                }
            }
        }

        if self.incomplete_len != 0 {
            return Ok(());
        }

        let remaining = &bytes[consumed..];
        let remaining_byte_offset = chunk_byte_offset
            .checked_add(u64::try_from(consumed).map_err(|_| ScanError::OffsetOverflow)?)
            .ok_or(ScanError::OffsetOverflow)?;
        match std::str::from_utf8(remaining) {
            Ok(text) => self.inspect_valid_text(text, remaining_byte_offset),
            Err(error) if error.error_len().is_some() => {
                self.invalid_utf8 = true;
                Ok(())
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                let (valid, incomplete) = remaining.split_at(valid_up_to);
                let text = std::str::from_utf8(valid).map_err(|_| ScanError::OffsetOverflow)?;
                self.inspect_valid_text(text, remaining_byte_offset)?;

                if incomplete.len() > 3 {
                    self.invalid_utf8 = true;
                    return Ok(());
                }
                self.incomplete[..incomplete.len()].copy_from_slice(incomplete);
                self.incomplete_len = incomplete.len();
                self.incomplete_byte_offset = remaining_byte_offset
                    .checked_add(u64::try_from(valid_up_to).map_err(|_| ScanError::OffsetOverflow)?)
                    .ok_or(ScanError::OffsetOverflow)?;
                Ok(())
            }
        }
    }

    pub(crate) fn finish(mut self) -> Finding {
        if self.incomplete_len != 0 {
            self.invalid_utf8 = true;
        }

        if self.invalid_utf8 {
            return Finding::new(
                MechanismIdentity::new(MECHANISM_ID, UNICODE_VERSION),
                FindingStatus::Invalid,
                vec![Evidence::new(
                    "utf8_validation",
                    "failed: the complete artifact is malformed or incomplete UTF-8",
                )],
                vec![INVALID_UTF8_LIMITATION.to_owned()],
                vec![],
            );
        }

        let status = if self.total_occurrence_count == 0 {
            FindingStatus::Absent
        } else {
            FindingStatus::Present
        };
        let locations_truncated = self.total_occurrence_count
            > u64::try_from(RETAINED_LOCATION_LIMIT).expect("the location limit fits in u64");
        Finding::new(
            MechanismIdentity::new(MECHANISM_ID, UNICODE_VERSION),
            status,
            vec![
                Evidence::new("locations", locations_json(&self.locations)),
                Evidence::new("locations_truncated", locations_truncated.to_string()),
                Evidence::new(
                    "total_occurrence_count",
                    self.total_occurrence_count.to_string(),
                ),
            ],
            vec![VALID_INTERPRETATION_LIMITATION.to_owned()],
            vec![],
        )
    }

    fn inspect_valid_text(&mut self, text: &str, byte_offset: u64) -> Result<(), ScanError> {
        for (relative_byte_offset, value) in text.char_indices() {
            let code_point = u32::from(value);
            if is_default_ignorable(code_point) {
                self.total_occurrence_count = self
                    .total_occurrence_count
                    .checked_add(1)
                    .ok_or(ScanError::OffsetOverflow)?;
                if self.locations.len() < RETAINED_LOCATION_LIMIT {
                    let relative_byte_offset = u64::try_from(relative_byte_offset)
                        .map_err(|_| ScanError::OffsetOverflow)?;
                    self.locations.push(Location {
                        code_point,
                        byte_offset: byte_offset
                            .checked_add(relative_byte_offset)
                            .ok_or(ScanError::OffsetOverflow)?,
                        scalar_offset: self.scalar_offset,
                    });
                }
            }
            self.scalar_offset = self
                .scalar_offset
                .checked_add(1)
                .ok_or(ScanError::OffsetOverflow)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ScanError {
    OffsetOverflow,
}

fn is_default_ignorable(code_point: u32) -> bool {
    DICP_RANGES
        .iter()
        .any(|&(start, end)| start <= code_point && code_point <= end)
}

fn locations_json(locations: &[Location]) -> String {
    let mut value = String::from("[");
    for (index, location) in locations.iter().enumerate() {
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

fn canonical_code_point(code_point: u32) -> String {
    if code_point <= 0xFFFF {
        format!("U+{code_point:04X}")
    } else {
        format!("U+{code_point:X}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED_DICP_DATA: &str =
        include_str!("../tests/fixtures/unicode-default-ignorable-17.0.0.txt");

    #[test]
    fn production_membership_exactly_matches_the_frozen_fixture_ranges() {
        let expected = fixture_ranges();
        assert_eq!(DICP_RANGES.as_slice(), expected.as_slice());

        for code_point in 0..=0x10FFFF {
            let expected_member = expected
                .iter()
                .any(|&(start, end)| start <= code_point && code_point <= end);
            assert_eq!(
                is_default_ignorable(code_point),
                expected_member,
                "membership differs at U+{code_point:04X}"
            );
        }
    }

    #[test]
    fn production_membership_handles_range_boundaries_and_gaps() {
        let ranges = fixture_ranges();
        for &(start, end) in &ranges {
            assert!(is_default_ignorable(start));
            assert!(is_default_ignorable(end));
            if start > 0 {
                let previous = start - 1;
                if !ranges.iter().any(|&(a, b)| a <= previous && previous <= b) {
                    assert!(!is_default_ignorable(previous));
                }
            }
            if end < 0x10FFFF {
                let next = end + 1;
                if !ranges.iter().any(|&(a, b)| a <= next && next <= b) {
                    assert!(!is_default_ignorable(next));
                }
            }
        }
        for gap in [0x0000, 0x0301, 0x200A, 0x2010, 0xE1000, 0x10FFFF] {
            assert!(!is_default_ignorable(gap), "U+{gap:04X} is a gap");
        }
    }

    fn fixture_ranges() -> Vec<(u32, u32)> {
        PINNED_DICP_DATA
            .lines()
            .filter_map(|line| {
                let data = line.split('#').next()?.trim();
                let (code_points, property) = data.split_once(';')?;
                if property.trim() != "Default_Ignorable_Code_Point" {
                    return None;
                }
                let code_points = code_points.trim();
                let (start, end) = code_points
                    .split_once("..")
                    .unwrap_or((code_points, code_points));
                Some((
                    u32::from_str_radix(start.trim(), 16).expect("fixture start is valid"),
                    u32::from_str_radix(end.trim(), 16).expect("fixture end is valid"),
                ))
            })
            .collect()
    }
}
