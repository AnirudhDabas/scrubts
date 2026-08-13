use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const REPORT_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Present,
    Absent,
    Unknown,
    Unsupported,
    Invalid,
    NotApplicable,
}

impl FindingStatus {
    pub const ALL: [Self; 6] = [
        Self::Present,
        Self::Absent,
        Self::Unknown,
        Self::Unsupported,
        Self::Invalid,
        Self::NotApplicable,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::Invalid => "invalid",
            Self::NotApplicable => "not_applicable",
        }
    }
}

impl fmt::Display for FindingStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Ord for FindingStatus {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for FindingStatus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for FindingStatus {
    type Err = ParseFindingStatusError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "present" => Ok(Self::Present),
            "absent" => Ok(Self::Absent),
            "unknown" => Ok(Self::Unknown),
            "unsupported" => Ok(Self::Unsupported),
            "invalid" => Ok(Self::Invalid),
            "not_applicable" => Ok(Self::NotApplicable),
            _ => Err(ParseFindingStatusError {
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseFindingStatusError {
    value: String,
}

impl ParseFindingStatusError {
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParseFindingStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown finding status: {:?}", self.value)
    }
}

impl Error for ParseFindingStatusError {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    name: String,
    version: String,
}

impl ToolIdentity {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut digest = String::with_capacity(64);
        for byte in bytes {
            digest.push(char::from(HEX[usize::from(byte >> 4)]));
            digest.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Self(digest)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<Sha256Digest> for String {
    fn from(digest: Sha256Digest) -> Self {
        digest.0
    }
}

impl TryFrom<String> for Sha256Digest {
    type Error = ParseSha256DigestError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            Ok(Self(value))
        } else {
            Err(ParseSha256DigestError { value })
        }
    }
}

impl FromStr for Sha256Digest {
    type Err = ParseSha256DigestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_owned())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseSha256DigestError {
    value: String,
}

impl ParseSha256DigestError {
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParseSha256DigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("SHA-256 digest must contain exactly 64 lowercase hexadecimal characters")
    }
}

impl Error for ParseSha256DigestError {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIdentity {
    path: String,
    byte_length: u64,
    content_sha256: Sha256Digest,
}

impl ArtifactIdentity {
    pub fn new(path: impl Into<String>, byte_length: u64, content_sha256: Sha256Digest) -> Self {
        Self {
            path: path.into(),
            byte_length,
            content_sha256,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn content_sha256(&self) -> &Sha256Digest {
        &self.content_sha256
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismIdentity {
    id: String,
    version: String,
}

impl MechanismIdentity {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    name: String,
    value: String,
}

impl Evidence {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    mechanism: MechanismIdentity,
    status: FindingStatus,
    evidence: Vec<Evidence>,
    limitations: Vec<String>,
    assumptions: Vec<String>,
}

impl Finding {
    pub fn new(
        mechanism: MechanismIdentity,
        status: FindingStatus,
        mut evidence: Vec<Evidence>,
        mut limitations: Vec<String>,
        mut assumptions: Vec<String>,
    ) -> Self {
        evidence.sort();
        limitations.sort();
        assumptions.sort();
        Self {
            mechanism,
            status,
            evidence,
            limitations,
            assumptions,
        }
    }

    pub fn mechanism(&self) -> &MechanismIdentity {
        &self.mechanism
    }

    pub const fn status(&self) -> FindingStatus {
        self.status
    }

    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }

    pub fn assumptions(&self) -> &[String] {
        &self.assumptions
    }

    fn canonicalize(&mut self) {
        self.evidence.sort();
        self.limitations.sort();
        self.assumptions.sort();
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Report {
    schema_version: &'static str,
    tool: ToolIdentity,
    artifact: ArtifactIdentity,
    findings: Vec<Finding>,
    limitations: Vec<String>,
    assumptions: Vec<String>,
}

impl Report {
    pub fn new(
        tool: ToolIdentity,
        artifact: ArtifactIdentity,
        mut findings: Vec<Finding>,
        mut limitations: Vec<String>,
        mut assumptions: Vec<String>,
    ) -> Self {
        Self::canonicalize(&mut findings, &mut limitations, &mut assumptions);
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            tool,
            artifact,
            findings,
            limitations,
            assumptions,
        }
    }

    pub const fn schema_version(&self) -> &'static str {
        self.schema_version
    }

    pub fn tool(&self) -> &ToolIdentity {
        &self.tool
    }

    pub fn artifact(&self) -> &ArtifactIdentity {
        &self.artifact
    }

    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }

    pub fn assumptions(&self) -> &[String] {
        &self.assumptions
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(input: &str) -> Result<Self, ParseReportError> {
        let declared: DeclaredSchemaVersion =
            serde_json::from_str(input).map_err(ParseReportError::InvalidJson)?;
        if declared.schema_version != REPORT_SCHEMA_VERSION {
            return Err(ParseReportError::UnsupportedSchemaVersion {
                found: declared.schema_version,
            });
        }

        let wire: ReportWire =
            serde_json::from_str(input).map_err(ParseReportError::InvalidJson)?;
        debug_assert_eq!(wire.schema_version, REPORT_SCHEMA_VERSION);

        Ok(Self::new(
            wire.tool,
            wire.artifact,
            wire.findings,
            wire.limitations,
            wire.assumptions,
        ))
    }

    fn canonicalize(
        findings: &mut [Finding],
        limitations: &mut [String],
        assumptions: &mut [String],
    ) {
        for finding in findings.iter_mut() {
            finding.canonicalize();
        }
        findings.sort_by(|left, right| {
            mechanism_order(left.mechanism().id())
                .cmp(&mechanism_order(right.mechanism().id()))
                .then_with(|| left.cmp(right))
        });
        limitations.sort();
        assumptions.sort();
    }
}

fn mechanism_order(id: &str) -> u8 {
    match id {
        "unicode.bidi_control" => 0,
        "unicode.default_ignorable_code_point" => 1,
        "unicode.normalization.nfc_difference" => 2,
        "unicode.normalization.nfkc_difference" => 3,
        "c2pa.text_manifest_wrapper" => 4,
        "c2pa.manifest_store" => 5,
        "c2pa.manifest_validation" => 6,
        "c2pa.hard_binding" => 7,
        "c2pa.credential_trust" => 8,
        _ => 9,
    }
}

#[derive(Debug, Deserialize)]
struct DeclaredSchemaVersion {
    schema_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportWire {
    schema_version: String,
    tool: ToolIdentity,
    artifact: ArtifactIdentity,
    findings: Vec<Finding>,
    limitations: Vec<String>,
    assumptions: Vec<String>,
}

#[derive(Debug)]
pub enum ParseReportError {
    InvalidJson(serde_json::Error),
    UnsupportedSchemaVersion { found: String },
}

impl fmt::Display for ParseReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid report JSON: {error}"),
            Self::UnsupportedSchemaVersion { found } => {
                write!(formatter, "unsupported report schema version: {found:?}")
            }
        }
    }
}

impl Error for ParseReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidJson(error) => Some(error),
            Self::UnsupportedSchemaVersion { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn digest() -> Sha256Digest {
        ABC_SHA256.parse().expect("test digest is valid")
    }

    fn report(findings: Vec<Finding>) -> Report {
        Report::new(
            ToolIdentity::new("scrub", "0.1.0"),
            ArtifactIdentity::new("sample.txt", 3, digest()),
            findings,
            vec!["z limitation".to_owned(), "a limitation".to_owned()],
            vec!["z assumption".to_owned(), "a assumption".to_owned()],
        )
    }

    #[test]
    fn every_status_has_exact_contract_spelling() {
        let expected = [
            (FindingStatus::Present, "\"present\""),
            (FindingStatus::Absent, "\"absent\""),
            (FindingStatus::Unknown, "\"unknown\""),
            (FindingStatus::Unsupported, "\"unsupported\""),
            (FindingStatus::Invalid, "\"invalid\""),
            (FindingStatus::NotApplicable, "\"not_applicable\""),
        ];

        assert_eq!(FindingStatus::ALL.len(), expected.len());
        for (status, json) in expected {
            assert_eq!(
                serde_json::to_string(&status).expect("status serializes"),
                json
            );
            assert_eq!(
                serde_json::from_str::<FindingStatus>(json).expect("status parses"),
                status
            );
        }
    }

    #[test]
    fn unknown_invalid_and_unsupported_never_fall_back_to_absent() {
        for status in [
            FindingStatus::Unknown,
            FindingStatus::Invalid,
            FindingStatus::Unsupported,
        ] {
            let json = serde_json::to_string(&status).expect("status serializes");
            let parsed: FindingStatus = serde_json::from_str(&json).expect("status parses");
            assert_eq!(parsed, status);
            assert_ne!(parsed, FindingStatus::Absent);
        }

        for unknown in ["removed", "", "ABSENT", "future_status"] {
            assert!(unknown.parse::<FindingStatus>().is_err());
            let json = format!("\"{unknown}\"");
            assert!(serde_json::from_str::<FindingStatus>(&json).is_err());
        }
    }

    #[test]
    fn exact_report_json_is_versioned_and_canonical() {
        let findings = vec![
            Finding::new(
                MechanismIdentity::new("z.mechanism", "2"),
                FindingStatus::Unknown,
                vec![Evidence::new("z", "2"), Evidence::new("a", "1")],
                vec!["z".to_owned(), "a".to_owned()],
                vec![],
            ),
            Finding::new(
                MechanismIdentity::new("a.mechanism", "1"),
                FindingStatus::Present,
                vec![Evidence::new("raw", "value")],
                vec![],
                vec!["known input".to_owned()],
            ),
        ];
        let json = report(findings).to_json().expect("report serializes");

        assert_eq!(
            json,
            concat!(
                "{\"schema_version\":\"0.1\",",
                "\"tool\":{\"name\":\"scrub\",\"version\":\"0.1.0\"},",
                "\"artifact\":{\"path\":\"sample.txt\",\"byte_length\":3,",
                "\"content_sha256\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"},",
                "\"findings\":[",
                "{\"mechanism\":{\"id\":\"a.mechanism\",\"version\":\"1\"},",
                "\"status\":\"present\",\"evidence\":[{\"name\":\"raw\",\"value\":\"value\"}],",
                "\"limitations\":[],\"assumptions\":[\"known input\"]},",
                "{\"mechanism\":{\"id\":\"z.mechanism\",\"version\":\"2\"},",
                "\"status\":\"unknown\",\"evidence\":[{\"name\":\"a\",\"value\":\"1\"},",
                "{\"name\":\"z\",\"value\":\"2\"}],\"limitations\":[\"a\",\"z\"],",
                "\"assumptions\":[]}],",
                "\"limitations\":[\"a limitation\",\"z limitation\"],",
                "\"assumptions\":[\"a assumption\",\"z assumption\"]}"
            )
        );
    }

    #[test]
    fn report_round_trip_normalizes_input_order() {
        let input = concat!(
            "{\"schema_version\":\"0.1\",",
            "\"tool\":{\"name\":\"scrub\",\"version\":\"0.1.0\"},",
            "\"artifact\":{\"path\":\"sample.txt\",\"byte_length\":3,",
            "\"content_sha256\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"},",
            "\"findings\":[],\"limitations\":[\"z\",\"a\"],",
            "\"assumptions\":[\"z\",\"a\"]}"
        );

        let parsed = Report::from_json(input).expect("report parses");
        let canonical = parsed.to_json().expect("report serializes");
        assert_eq!(canonical, input.replace("[\"z\",\"a\"]", "[\"a\",\"z\"]"));
        assert_eq!(
            Report::from_json(&canonical).expect("canonical report parses"),
            parsed
        );
    }

    #[test]
    fn production_mechanisms_have_the_frozen_unicode_then_c2pa_order() {
        let ids = [
            "c2pa.credential_trust",
            "unicode.normalization.nfkc_difference",
            "c2pa.manifest_store",
            "unicode.bidi_control",
            "c2pa.text_manifest_wrapper",
            "unicode.default_ignorable_code_point",
            "c2pa.hard_binding",
            "unicode.normalization.nfc_difference",
            "c2pa.manifest_validation",
        ];
        let findings = ids
            .into_iter()
            .map(|id| {
                Finding::new(
                    MechanismIdentity::new(id, "1"),
                    FindingStatus::Unknown,
                    vec![],
                    vec![],
                    vec![],
                )
            })
            .collect();

        let report = report(findings);
        let actual: Vec<_> = report
            .findings()
            .iter()
            .map(|finding| finding.mechanism().id())
            .collect();
        assert_eq!(
            actual,
            [
                "unicode.bidi_control",
                "unicode.default_ignorable_code_point",
                "unicode.normalization.nfc_difference",
                "unicode.normalization.nfkc_difference",
                "c2pa.text_manifest_wrapper",
                "c2pa.manifest_store",
                "c2pa.manifest_validation",
                "c2pa.hard_binding",
                "c2pa.credential_trust",
            ]
        );
    }

    #[test]
    fn unsupported_schema_version_is_typed() {
        let json = report(vec![])
            .to_json()
            .expect("report serializes")
            .replace("\"schema_version\":\"0.1\"", "\"schema_version\":\"9\"");

        assert!(matches!(
            Report::from_json(&json),
            Err(ParseReportError::UnsupportedSchemaVersion { found }) if found == "9"
        ));
    }

    #[test]
    fn unsupported_schema_version_precedes_v01_body_validation() {
        let json = r#"{"schema_version":"9","not_valid_under_v01":true}"#;

        assert!(matches!(
            Report::from_json(json),
            Err(ParseReportError::UnsupportedSchemaVersion { found }) if found == "9"
        ));
    }

    #[test]
    fn finding_without_status_is_rejected() {
        let finding = Finding::new(
            MechanismIdentity::new("example", "1"),
            FindingStatus::Unknown,
            vec![],
            vec![],
            vec![],
        );
        let json = report(vec![finding])
            .to_json()
            .expect("report serializes")
            .replace("\"status\":\"unknown\",", "");

        assert!(matches!(
            Report::from_json(&json),
            Err(ParseReportError::InvalidJson(_))
        ));
    }

    #[test]
    fn malformed_hash_and_unknown_fields_are_rejected() {
        let json = report(vec![]).to_json().expect("report serializes");
        let malformed_hash = json.replace(ABC_SHA256, "ABC");
        assert!(matches!(
            Report::from_json(&malformed_hash),
            Err(ParseReportError::InvalidJson(_))
        ));

        let unknown_field = json.replace(
            "\"schema_version\":\"0.1\"",
            "\"schema_version\":\"0.1\",\"future\":true",
        );
        assert!(matches!(
            Report::from_json(&unknown_field),
            Err(ParseReportError::InvalidJson(_))
        ));
    }
}
