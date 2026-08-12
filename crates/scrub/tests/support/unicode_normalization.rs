use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub(crate) const UNICODE_MAX: u32 = 0x10_FFFF;
pub(crate) const SURROGATE_START: u32 = 0xD800;
pub(crate) const SURROGATE_END: u32 = 0xDFFF;

pub(crate) const NORMALIZATION_TEST_BYTES: &[u8] =
    include_bytes!("../fixtures/NormalizationTest-17.0.0.txt");
pub(crate) const DERIVED_AGE_BYTES: &[u8] = include_bytes!("../fixtures/DerivedAge-17.0.0.txt");

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

pub(crate) const NORMALIZATION_FORMS: [NormalizationForm; 4] = [
    NormalizationForm::Nfc,
    NormalizationForm::Nfd,
    NormalizationForm::Nfkc,
    NormalizationForm::Nfkd,
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct NormalizationRecord {
    pub(crate) part: u8,
    pub(crate) columns: [Vec<u32>; 5],
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExpectedTransformation<'a> {
    pub(crate) form: NormalizationForm,
    pub(crate) input_column: usize,
    pub(crate) expected_column: usize,
    pub(crate) input: &'a [u32],
    pub(crate) expected: &'a [u32],
}

impl NormalizationRecord {
    pub(crate) fn expected_transformations(
        &self,
    ) -> impl Iterator<Item = ExpectedTransformation<'_>> {
        RELATIONSHIPS
            .into_iter()
            .map(
                |(form, input_column, expected_column)| ExpectedTransformation {
                    form,
                    input_column,
                    expected_column,
                    input: &self.columns[input_column],
                    expected: &self.columns[expected_column],
                },
            )
    }
}

const RELATIONSHIPS: [(NormalizationForm, usize, usize); 20] = [
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
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct DerivedAgeRecord {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) version: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ParseError {
    line: usize,
    message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

impl Error for ParseError {}

pub(crate) fn parse_normalization_test(
    bytes: &[u8],
) -> Result<Vec<NormalizationRecord>, ParseError> {
    let input = std::str::from_utf8(bytes)
        .map_err(|error| ParseError::new(1, format!("fixture is not UTF-8: {error}")))?;
    reject_carriage_returns(input)?;
    let mut records = Vec::new();
    let mut current_part = None;
    let mut seen_parts = [false; 6];

    for (line_index, line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let data = line
            .split_once('#')
            .map_or(line, |(before, _)| before)
            .trim();
        if data.is_empty() {
            continue;
        }

        if let Some(suffix) = data.strip_prefix("@Part") {
            let part = parse_part(suffix, line_number)?;
            if seen_parts[usize::from(part)] {
                return Err(ParseError::new(line_number, "duplicate Part directive"));
            }
            let expected_part = seen_parts.iter().filter(|seen| **seen).count();
            if usize::from(part) != expected_part {
                return Err(ParseError::new(
                    line_number,
                    format!("expected Part {expected_part}, found Part {part}"),
                ));
            }
            seen_parts[usize::from(part)] = true;
            current_part = Some(part);
            continue;
        }
        if data.starts_with('@') {
            return Err(ParseError::new(line_number, "unrecognized directive"));
        }

        let part = current_part
            .ok_or_else(|| ParseError::new(line_number, "data record precedes Part 0"))?;
        let fields: Vec<_> = data.split(';').collect();
        if fields.len() != 6 || !fields[5].trim().is_empty() {
            return Err(ParseError::new(
                line_number,
                "data record must have five fields and a trailing semicolon",
            ));
        }

        let mut parsed_columns = Vec::with_capacity(5);
        for field in &fields[..5] {
            let tokens: Vec<_> = field.split_ascii_whitespace().collect();
            if tokens.is_empty() {
                return Err(ParseError::new(line_number, "semantic field is empty"));
            }
            let mut sequence = Vec::with_capacity(tokens.len());
            for token in tokens {
                sequence.push(parse_code_point(token, line_number)?);
            }
            parsed_columns.push(sequence);
        }
        let columns = parsed_columns
            .try_into()
            .map_err(|_| ParseError::new(line_number, "record does not have five columns"))?;
        records.push(NormalizationRecord { part, columns });
    }

    if !seen_parts.into_iter().all(|seen| seen) {
        return Err(ParseError::new(
            input.lines().count().max(1),
            "fixture does not contain every Part directive from 0 through 5",
        ));
    }
    Ok(records)
}

pub(crate) fn parse_derived_age(bytes: &[u8]) -> Result<Vec<DerivedAgeRecord>, ParseError> {
    let input = std::str::from_utf8(bytes)
        .map_err(|error| ParseError::new(1, format!("fixture is not UTF-8: {error}")))?;
    reject_carriage_returns(input)?;
    let mut records = Vec::new();

    for (line_index, line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let data = line
            .split_once('#')
            .map_or(line, |(before, _)| before)
            .trim();
        if data.is_empty() {
            continue;
        }
        let fields: Vec<_> = data.split(';').collect();
        if fields.len() != 2 {
            return Err(ParseError::new(
                line_number,
                "Age record must contain one semicolon",
            ));
        }
        let (start, end) = parse_range(fields[0].trim(), line_number)?;
        let version = fields[1].trim();
        if !valid_age_version(version) {
            return Err(ParseError::new(line_number, "invalid Age version"));
        }
        records.push(DerivedAgeRecord {
            start,
            end,
            version: version.to_owned(),
        });
    }
    Ok(records)
}

pub(crate) fn assigned_code_points(
    records: &[DerivedAgeRecord],
) -> Result<BTreeSet<u32>, ParseError> {
    let mut assigned = BTreeSet::new();
    for record in records {
        for code_point in record.start..=record.end {
            if !assigned.insert(code_point) {
                return Err(ParseError::new(
                    1,
                    format!("overlapping Age records at U+{code_point:04X}"),
                ));
            }
        }
    }
    Ok(assigned)
}

pub(crate) fn part1_source_code_points(records: &[NormalizationRecord]) -> BTreeSet<u32> {
    records
        .iter()
        .filter(|record| record.part == 1)
        .flat_map(|record| record.columns[0].iter().copied())
        .collect()
}

pub(crate) fn assigned_complement(
    assigned: &BTreeSet<u32>,
    part1_sources: &BTreeSet<u32>,
) -> BTreeSet<u32> {
    assigned.difference(part1_sources).copied().collect()
}

pub(crate) fn is_unicode_scalar(code_point: u32) -> bool {
    code_point <= UNICODE_MAX && !(SURROGATE_START..=SURROGATE_END).contains(&code_point)
}

pub(crate) fn unicode_scalar_values() -> impl Iterator<Item = u32> {
    (0..=UNICODE_MAX).filter(|code_point| is_unicode_scalar(*code_point))
}

pub(crate) fn unassigned_scalar_values<'a>(
    assigned: &'a BTreeSet<u32>,
) -> impl Iterator<Item = u32> + 'a {
    unicode_scalar_values().filter(|code_point| !assigned.contains(code_point))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct IdentityExpectation {
    pub(crate) form: NormalizationForm,
    pub(crate) input: u32,
    pub(crate) expected: u32,
}

pub(crate) fn identity_expectations(
    code_points: impl IntoIterator<Item = u32>,
) -> impl Iterator<Item = IdentityExpectation> {
    code_points.into_iter().flat_map(|code_point| {
        NORMALIZATION_FORMS
            .into_iter()
            .map(move |form| IdentityExpectation {
                form,
                input: code_point,
                expected: code_point,
            })
    })
}

fn parse_part(suffix: &str, line_number: usize) -> Result<u8, ParseError> {
    if suffix.len() != 1 || !suffix.as_bytes()[0].is_ascii_digit() {
        return Err(ParseError::new(line_number, "invalid Part directive"));
    }
    let part = suffix.as_bytes()[0] - b'0';
    if part > 5 {
        return Err(ParseError::new(
            line_number,
            "Part number is outside 0 through 5",
        ));
    }
    Ok(part)
}

fn parse_range(value: &str, line_number: usize) -> Result<(u32, u32), ParseError> {
    let (start, end) = match value.split_once("..") {
        Some((start, end)) => (
            parse_code_point(start, line_number)?,
            parse_code_point(end, line_number)?,
        ),
        None => {
            let code_point = parse_code_point(value, line_number)?;
            (code_point, code_point)
        }
    };
    if start > end {
        return Err(ParseError::new(line_number, "descending code point range"));
    }
    Ok((start, end))
}

fn parse_code_point(token: &str, line_number: usize) -> Result<u32, ParseError> {
    if !(4..=6).contains(&token.len())
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    {
        return Err(ParseError::new(
            line_number,
            format!("invalid code point token {token:?}"),
        ));
    }
    let code_point = u32::from_str_radix(token, 16)
        .map_err(|_| ParseError::new(line_number, "invalid hexadecimal code point"))?;
    if code_point > UNICODE_MAX {
        return Err(ParseError::new(line_number, "code point exceeds U+10FFFF"));
    }
    Ok(code_point)
}

fn valid_age_version(version: &str) -> bool {
    let Some((major, minor)) = version.split_once('.') else {
        return false;
    };
    if major.is_empty()
        || minor.is_empty()
        || !major.bytes().all(|byte| byte.is_ascii_digit())
        || !minor.bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }

    matches!(major.parse::<u8>(), Ok(1..=u8::MAX)) && minor.parse::<u8>().is_ok()
}

fn reject_carriage_returns(input: &str) -> Result<(), ParseError> {
    if let Some(byte_index) = input.find('\r') {
        let line_number = input[..byte_index]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        return Err(ParseError::new(
            line_number,
            "carriage return is not permitted",
        ));
    }
    Ok(())
}
