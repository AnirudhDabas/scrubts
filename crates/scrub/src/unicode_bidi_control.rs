use std::fmt::Write as _;

use scrub_report::{Evidence, Finding, FindingStatus, MechanismIdentity};

use crate::utf8_stream::ScalarObservation;

pub(crate) const MECHANISM_ID: &str = "unicode.bidi_control";
pub(crate) const UNICODE_VERSION: &str = "17.0.0";

const RETAINED_LOCATION_LIMIT: usize = 256;
const VALID_INTERPRETATION_LIMITATION: &str = "Bidi_Control presence is a neutral Unicode property observation; directional-formatting controls may have legitimate uses.";
const INVALID_UTF8_LIMITATION: &str = "Bidi_Control occurrence evidence is unavailable because the complete artifact is not valid UTF-8.";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ControlIdentity {
    code_point: u32,
    abbreviation: &'static str,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Location {
    identity: ControlIdentity,
    byte_offset: u64,
    scalar_offset: u64,
}

pub(crate) struct Inspection {
    total_occurrence_count: u64,
    locations: Vec<Location>,
}

impl Inspection {
    pub(crate) fn new() -> Self {
        Self {
            total_occurrence_count: 0,
            locations: Vec::with_capacity(RETAINED_LOCATION_LIMIT),
        }
    }

    pub(crate) fn observe(&mut self, observation: ScalarObservation) -> Result<(), ScanError> {
        let Some(identity) = control_identity(u32::from(observation.value)) else {
            return Ok(());
        };

        self.total_occurrence_count = self
            .total_occurrence_count
            .checked_add(1)
            .ok_or(ScanError::OffsetOverflow)?;
        if self.locations.len() < RETAINED_LOCATION_LIMIT {
            self.locations.push(Location {
                identity,
                byte_offset: observation.byte_offset,
                scalar_offset: observation.scalar_offset,
            });
        }
        Ok(())
    }

    pub(crate) fn finish(self, valid_utf8: bool) -> Finding {
        if !valid_utf8 {
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
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ScanError {
    OffsetOverflow,
}

fn control_identity(code_point: u32) -> Option<ControlIdentity> {
    CONTROL_IDENTITIES
        .iter()
        .copied()
        .find(|identity| identity.code_point == code_point)
}

fn locations_json(locations: &[Location]) -> String {
    let mut value = String::from("[");
    for (index, location) in locations.iter().enumerate() {
        if index != 0 {
            value.push(',');
        }
        write!(
            value,
            "{{\"code_point\":\"{}\",\"abbreviation\":\"{}\",\"byte_offset\":{},\"scalar_offset\":{}}}",
            canonical_code_point(location.identity.code_point),
            location.identity.abbreviation,
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

    const PINNED_BIDI_CONTROL_DATA: &str =
        include_str!("../tests/fixtures/unicode-bidi-control-17.0.0.txt");

    #[test]
    fn production_membership_exactly_matches_the_frozen_ranges_for_every_scalar() {
        let expected = fixture_ranges();
        let mut production_member_count = 0;

        for code_point in 0..=0x10FFFF {
            if char::from_u32(code_point).is_none() {
                continue;
            }
            let expected_member = expected
                .iter()
                .any(|&(start, end)| start <= code_point && code_point <= end);
            let production_member = control_identity(code_point).is_some();
            assert_eq!(
                production_member, expected_member,
                "membership differs at U+{code_point:04X}"
            );
            production_member_count += usize::from(production_member);
        }

        assert_eq!(production_member_count, 12);
    }

    #[test]
    fn identities_and_range_neighbors_are_exact() {
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
        let actual: Vec<_> = CONTROL_IDENTITIES
            .iter()
            .map(|identity| (identity.code_point, identity.abbreviation))
            .collect();
        assert_eq!(actual, expected);

        for nonmember in [
            0x061B, 0x061D, 0x200D, 0x2010, 0x2029, 0x202F, 0x2065, 0x206A,
        ] {
            assert!(control_identity(nonmember).is_none(), "U+{nonmember:04X}");
        }
    }

    fn fixture_ranges() -> Vec<(u32, u32)> {
        PINNED_BIDI_CONTROL_DATA
            .lines()
            .filter_map(|line| {
                let data = line.split('#').next()?.trim();
                let (code_points, property) = data.split_once(';')?;
                if property.trim() != "Bidi_Control" {
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
