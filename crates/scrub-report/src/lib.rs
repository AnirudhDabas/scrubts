use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Returns whether a scalar must be represented visibly in human output.
///
/// This covers terminal controls, layout-changing separators, and the Unicode
/// bidirectional formatting controls that can reorder surrounding labels.
pub fn is_forbidden_human_control(scalar: char) -> bool {
    scalar.is_control()
        || matches!(
            scalar,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// Encodes untrusted text for a plain human terminal projection.
pub fn human_safe(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for scalar in value.chars() {
        if is_forbidden_human_control(scalar) {
            use std::fmt::Write as _;
            write!(output, "\\u{{{:x}}}", u32::from(scalar))
                .expect("writing to a String cannot fail");
        } else {
            output.push(scalar);
        }
    }
    output
}

pub const REPORT_SCHEMA_VERSION: &str = "0.2";
pub const SEMANTIC_REPORT_SCHEMA_VERSION: &str = "0.1";

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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    UnicodePropertyMembership,
    UnicodeNormalizationDifference,
    C2paTextManifestWrapper,
    C2paManifestStore,
    C2paManifestValidation,
    C2paHardBinding,
    C2paCredentialTrust,
    StatisticalWatermarkDecision,
    ProviderTextWatermark,
    Unclassified,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorityClass {
    Standard,
    PublicReference,
    PublicMechanismPrivateKey,
    UndocumentedProvider,
    ProjectImplementation,
    Unspecified,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceRelationship {
    RelatedFamilyNotDeploymentParity,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedReference {
    mechanism_id: String,
    relationship: ReferenceRelationship,
}

impl RelatedReference {
    pub fn mechanism_id(&self) -> &str {
        &self.mechanism_id
    }

    pub const fn relationship(&self) -> ReferenceRelationship {
        self.relationship
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifierIdentity {
    id: String,
    version: String,
    availability: VerifierAvailability,
}

impl VerifierIdentity {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub const fn availability(&self) -> VerifierAvailability {
        self.availability
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityTrace {
    mechanism: AuthorityClass,
    implementation: AuthorityClass,
    detector: AuthorityClass,
    source_ids: Vec<String>,
    related_reference: Option<RelatedReference>,
}

impl AuthorityTrace {
    pub const fn mechanism(&self) -> AuthorityClass {
        self.mechanism
    }

    pub const fn implementation(&self) -> AuthorityClass {
        self.implementation
    }

    pub const fn detector(&self) -> AuthorityClass {
        self.detector
    }

    pub fn source_ids(&self) -> &[String] {
        &self.source_ids
    }

    pub const fn related_reference(&self) -> Option<&RelatedReference> {
        self.related_reference.as_ref()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceId {
    UnicodePropertyPresent,
    UnicodePropertyAbsentWithinScope,
    UnicodeNormalizationDifferencePresent,
    UnicodeNormalizationDifferenceAbsentWithinScope,
    C2paCarrierPresent,
    C2paCarrierAbsentWithinScope,
    C2paManifestStorePresent,
    C2paManifestStoreAbsentWithinScope,
    C2paClaimValid,
    C2paHardBindingValid,
    C2paCredentialTrusted,
    ProviderMechanismFamilyDisclosed,
    ProviderDetectorUnavailable,
    PublicReferenceWatermarkPresent,
    PublicReferenceWatermarkAbsentWithinConfiguration,
    ArtifactClean,
    HumanAuthorship,
    AiAuthorship,
    MaliciousIntent,
    WatermarkPresent,
    WatermarkAbsent,
    C2paClaimValidity,
    C2paHardBinding,
    C2paCredentialTrust,
    Authorship,
    Truthfulness,
    ClaudeWatermarkPresent,
    ClaudeWatermarkAbsent,
    ClaudeProviderParity,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproduction {
    command: Vec<String>,
}

impl Reproduction {
    pub fn command(&self) -> &[String] {
        &self.command
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofTrace {
    observation: ObservationKind,
    verifier: VerifierIdentity,
    authority: AuthorityTrace,
    configuration_identity: Option<String>,
    supports: Vec<InferenceId>,
    does_not_support: Vec<InferenceId>,
    reproduce: Reproduction,
}

impl ProofTrace {
    pub const fn observation(&self) -> ObservationKind {
        self.observation
    }

    pub fn verifier(&self) -> &VerifierIdentity {
        &self.verifier
    }

    pub fn authority(&self) -> &AuthorityTrace {
        &self.authority
    }

    pub fn configuration_identity(&self) -> Option<&str> {
        self.configuration_identity.as_deref()
    }

    pub fn supports(&self) -> &[InferenceId] {
        &self.supports
    }

    pub fn does_not_support(&self) -> &[InferenceId] {
        &self.does_not_support
    }

    pub fn reproduce(&self) -> &Reproduction {
        &self.reproduce
    }
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReportConstructionError {
    InvalidMechanismStatus {
        mechanism_id: String,
        status: FindingStatus,
    },
    DuplicateMechanism {
        mechanism_id: String,
    },
}

impl fmt::Display for ReportConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMechanismStatus {
                mechanism_id,
                status,
            } => write!(
                formatter,
                "mechanism {mechanism_id:?} cannot have finding status {status}"
            ),
            Self::DuplicateMechanism { mechanism_id } => {
                write!(
                    formatter,
                    "report contains duplicate mechanism {mechanism_id:?}"
                )
            }
        }
    }
}

impl Error for ReportConstructionError {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    mechanism: MechanismIdentity,
    status: FindingStatus,
    trace: ProofTrace,
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
    ) -> Result<Self, ReportConstructionError> {
        if !valid_mechanism_status(mechanism.id(), status) {
            return Err(ReportConstructionError::InvalidMechanismStatus {
                mechanism_id: mechanism.id().to_owned(),
                status,
            });
        }
        evidence.sort();
        limitations.sort();
        assumptions.sort();
        let trace = proof_trace(&mechanism, status);
        Ok(Self {
            mechanism,
            status,
            trace,
            evidence,
            limitations,
            assumptions,
        })
    }

    pub fn mechanism(&self) -> &MechanismIdentity {
        &self.mechanism
    }

    pub const fn status(&self) -> FindingStatus {
        self.status
    }

    pub fn trace(&self) -> &ProofTrace {
        &self.trace
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
        self.trace.authority.source_ids.sort();
        self.trace.supports.sort();
        self.trace.does_not_support.sort();
        self.evidence.sort();
        self.limitations.sort();
        self.assumptions.sort();
    }
}

fn valid_mechanism_status(mechanism_id: &str, status: FindingStatus) -> bool {
    match mechanism_id {
        // The frozen authority record has no runnable Claude provider detector.
        "anthropic.embedded_text_watermark" => {
            matches!(
                status,
                FindingStatus::Unknown | FindingStatus::NotApplicable
            )
        }
        _ => true,
    }
}

fn proof_trace(mechanism: &MechanismIdentity, status: FindingStatus) -> ProofTrace {
    let generic_boundaries = [
        InferenceId::AiAuthorship,
        InferenceId::ArtifactClean,
        InferenceId::HumanAuthorship,
    ];
    let command = Reproduction {
        command: vec![
            "scrub".to_owned(),
            "inspect".to_owned(),
            "<artifact>".to_owned(),
            "--json".to_owned(),
            "--explain".to_owned(),
        ],
    };
    let mk = |observation,
              verifier_id: &str,
              verifier_version: &str,
              availability,
              mechanism_authority,
              implementation_authority,
              detector_authority,
              source_ids: &[&str],
              configuration_identity: Option<&str>,
              supports: Vec<InferenceId>,
              mut does_not_support: Vec<InferenceId>,
              related_reference| {
        does_not_support.extend(generic_boundaries.iter().copied());
        does_not_support.sort();
        does_not_support.dedup();
        let mut supports = supports;
        supports.sort();
        supports.dedup();
        let mut source_ids: Vec<_> = source_ids.iter().map(|value| (*value).to_owned()).collect();
        source_ids.sort();
        source_ids.dedup();
        ProofTrace {
            observation,
            verifier: VerifierIdentity {
                id: verifier_id.to_owned(),
                version: verifier_version.to_owned(),
                availability,
            },
            authority: AuthorityTrace {
                mechanism: mechanism_authority,
                implementation: implementation_authority,
                detector: detector_authority,
                source_ids,
                related_reference,
            },
            configuration_identity: configuration_identity.map(str::to_owned),
            supports,
            does_not_support,
            reproduce: command.clone(),
        }
    };

    match mechanism.id() {
        "unicode.bidi_control" | "unicode.default_ignorable_code_point" => {
            let supports = match status {
                FindingStatus::Present => vec![InferenceId::UnicodePropertyPresent],
                FindingStatus::Absent => vec![InferenceId::UnicodePropertyAbsentWithinScope],
                _ => vec![],
            };
            mk(
                ObservationKind::UnicodePropertyMembership,
                "scrub.unicode_property_scan",
                mechanism.version(),
                VerifierAvailability::Available,
                AuthorityClass::Standard,
                AuthorityClass::ProjectImplementation,
                AuthorityClass::ProjectImplementation,
                if mechanism.id() == "unicode.bidi_control" {
                    &["unicode-uax44-17.0.0", "unicode-proplist-17.0.0"]
                } else {
                    &[
                        "unicode-uax44-17.0.0",
                        "unicode-derived-core-properties-17.0.0",
                    ]
                },
                Some("unicode-17.0.0"),
                supports,
                vec![
                    InferenceId::MaliciousIntent,
                    InferenceId::WatermarkAbsent,
                    InferenceId::WatermarkPresent,
                ],
                None,
            )
        }
        "unicode.normalization.nfc_difference" | "unicode.normalization.nfkc_difference" => {
            let supports = match status {
                FindingStatus::Present => {
                    vec![InferenceId::UnicodeNormalizationDifferencePresent]
                }
                FindingStatus::Absent => {
                    vec![InferenceId::UnicodeNormalizationDifferenceAbsentWithinScope]
                }
                _ => vec![],
            };
            mk(
                ObservationKind::UnicodeNormalizationDifference,
                "unicode-normalization",
                "0.1.25",
                VerifierAvailability::Available,
                AuthorityClass::Standard,
                AuthorityClass::PublicReference,
                AuthorityClass::PublicReference,
                &[
                    "unicode-uax15-17.0.0",
                    "unicode-normalization-crate-0.1.25",
                    "unicode-normalization-test-17.0.0",
                ],
                Some("unicode-17.0.0"),
                supports,
                vec![
                    InferenceId::MaliciousIntent,
                    InferenceId::WatermarkAbsent,
                    InferenceId::WatermarkPresent,
                ],
                None,
            )
        }
        "c2pa.text_manifest_wrapper" => {
            let supports = match status {
                FindingStatus::Present => vec![InferenceId::C2paCarrierPresent],
                FindingStatus::Absent => vec![InferenceId::C2paCarrierAbsentWithinScope],
                _ => vec![],
            };
            mk(
                ObservationKind::C2paTextManifestWrapper,
                "scrub.c2pa_text_wrapper",
                mechanism.version(),
                VerifierAvailability::Available,
                AuthorityClass::Standard,
                AuthorityClass::ProjectImplementation,
                AuthorityClass::ProjectImplementation,
                &["c2pa-spec-2.4"],
                Some("c2pa-2.4-a8"),
                supports,
                vec![
                    InferenceId::Authorship,
                    InferenceId::C2paClaimValidity,
                    InferenceId::C2paCredentialTrust,
                    InferenceId::C2paHardBinding,
                    InferenceId::Truthfulness,
                    InferenceId::WatermarkAbsent,
                    InferenceId::WatermarkPresent,
                ],
                None,
            )
        }
        "c2pa.manifest_store"
        | "c2pa.manifest_validation"
        | "c2pa.hard_binding"
        | "c2pa.credential_trust" => {
            let (observation, supports, boundaries) = match mechanism.id() {
                "c2pa.manifest_store" => (
                    ObservationKind::C2paManifestStore,
                    match status {
                        FindingStatus::Present => vec![InferenceId::C2paManifestStorePresent],
                        FindingStatus::Absent => {
                            vec![InferenceId::C2paManifestStoreAbsentWithinScope]
                        }
                        _ => vec![],
                    },
                    vec![
                        InferenceId::C2paClaimValidity,
                        InferenceId::C2paHardBinding,
                        InferenceId::C2paCredentialTrust,
                    ],
                ),
                "c2pa.manifest_validation" => (
                    ObservationKind::C2paManifestValidation,
                    if status == FindingStatus::Present {
                        vec![InferenceId::C2paClaimValid]
                    } else {
                        vec![]
                    },
                    vec![
                        InferenceId::C2paHardBinding,
                        InferenceId::C2paCredentialTrust,
                    ],
                ),
                "c2pa.hard_binding" => (
                    ObservationKind::C2paHardBinding,
                    if status == FindingStatus::Present {
                        vec![InferenceId::C2paHardBindingValid]
                    } else {
                        vec![]
                    },
                    vec![InferenceId::C2paCredentialTrust],
                ),
                _ => (
                    ObservationKind::C2paCredentialTrust,
                    if status == FindingStatus::Present {
                        vec![InferenceId::C2paCredentialTrusted]
                    } else {
                        vec![]
                    },
                    vec![],
                ),
            };
            let mut boundaries = boundaries;
            boundaries.extend([
                InferenceId::Authorship,
                InferenceId::Truthfulness,
                InferenceId::WatermarkAbsent,
                InferenceId::WatermarkPresent,
            ]);
            mk(
                observation,
                "c2pa-rs",
                "0.90.12",
                VerifierAvailability::Available,
                AuthorityClass::Standard,
                AuthorityClass::PublicReference,
                AuthorityClass::PublicReference,
                &["c2pa-spec-2.4", "c2pa-rs"],
                Some("c2pa-rs-0.90.12-fixed-local-settings"),
                supports,
                boundaries,
                None,
            )
        }
        "reference.synthid_text" => {
            let supports = match status {
                FindingStatus::Present => vec![InferenceId::PublicReferenceWatermarkPresent],
                FindingStatus::Absent => {
                    vec![InferenceId::PublicReferenceWatermarkAbsentWithinConfiguration]
                }
                _ => vec![],
            };
            mk(
                ObservationKind::StatisticalWatermarkDecision,
                "waterlarp.reference_synthid_text",
                "transformers-v5.15.0@5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9",
                VerifierAvailability::Available,
                AuthorityClass::PublicReference,
                AuthorityClass::PublicReference,
                AuthorityClass::PublicReference,
                &["synthid-text", "synthid-text-transformers"],
                Some("recorded-controlled-configuration-required"),
                supports,
                vec![
                    InferenceId::ClaudeProviderParity,
                    InferenceId::ClaudeWatermarkAbsent,
                    InferenceId::ClaudeWatermarkPresent,
                ],
                None,
            )
        }
        "anthropic.embedded_text_watermark" => mk(
            ObservationKind::ProviderTextWatermark,
            "anthropic.provider_detector",
            "unpublished",
            VerifierAvailability::Unavailable,
            AuthorityClass::PublicMechanismPrivateKey,
            AuthorityClass::UndocumentedProvider,
            AuthorityClass::UndocumentedProvider,
            &[
                "anthropic-claude-marking",
                "anthropic-claude-text-watermark",
            ],
            None,
            vec![
                InferenceId::ProviderDetectorUnavailable,
                InferenceId::ProviderMechanismFamilyDisclosed,
            ],
            vec![
                InferenceId::ClaudeProviderParity,
                InferenceId::ClaudeWatermarkAbsent,
                InferenceId::ClaudeWatermarkPresent,
                InferenceId::WatermarkAbsent,
                InferenceId::WatermarkPresent,
            ],
            Some(RelatedReference {
                mechanism_id: "reference.synthid_text".to_owned(),
                relationship: ReferenceRelationship::RelatedFamilyNotDeploymentParity,
            }),
        ),
        _ => mk(
            ObservationKind::Unclassified,
            "unspecified",
            mechanism.version(),
            VerifierAvailability::Unavailable,
            AuthorityClass::Unspecified,
            AuthorityClass::Unspecified,
            AuthorityClass::Unspecified,
            &[],
            None,
            vec![],
            vec![],
            None,
        ),
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

/// A report decoded from external JSON.
///
/// This wrapper records the trust boundary explicitly: parsing proves only
/// structural and frozen-ontology consistency. It does not authenticate the
/// producer or independently reproduce any observation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UntrustedReport {
    report: Report,
}

impl UntrustedReport {
    pub fn as_report(&self) -> &Report {
        &self.report
    }

    pub fn into_report(self) -> Report {
        self.report
    }
}

impl Report {
    pub fn new(
        tool: ToolIdentity,
        artifact: ArtifactIdentity,
        mut findings: Vec<Finding>,
        mut limitations: Vec<String>,
        mut assumptions: Vec<String>,
    ) -> Result<Self, ReportConstructionError> {
        for finding in &findings {
            if !valid_mechanism_status(finding.mechanism.id(), finding.status) {
                return Err(ReportConstructionError::InvalidMechanismStatus {
                    mechanism_id: finding.mechanism.id.clone(),
                    status: finding.status,
                });
            }
        }
        let mut mechanism_ids = std::collections::BTreeSet::new();
        for finding in &findings {
            if !mechanism_ids.insert(finding.mechanism.id.as_str()) {
                return Err(ReportConstructionError::DuplicateMechanism {
                    mechanism_id: finding.mechanism.id.clone(),
                });
            }
        }
        Self::canonicalize(&mut findings, &mut limitations, &mut assumptions);
        Ok(Self {
            schema_version: REPORT_SCHEMA_VERSION,
            tool,
            artifact,
            findings,
            limitations,
            assumptions,
        })
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

    /// Returns scrub's deterministic, path-free semantic JSON projection.
    ///
    /// This is a project-specific serialization contract, not RFC 8785/JCS.
    pub fn canonical_report_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&SemanticReport {
            schema_version: SEMANTIC_REPORT_SCHEMA_VERSION,
            tool: &self.tool,
            artifact: SemanticArtifactIdentity {
                byte_length: self.artifact.byte_length,
                content_sha256: &self.artifact.content_sha256,
            },
            findings: &self.findings,
            limitations: &self.limitations,
            assumptions: &self.assumptions,
        })
    }

    /// Parses an externally supplied report as an [`UntrustedReport`].
    ///
    /// Successful parsing is not authentication and is not independent
    /// reproduction of the report's evidence.
    pub fn from_json(input: &str) -> Result<UntrustedReport, ParseReportError> {
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

        for finding in &wire.findings {
            if finding.trace != proof_trace(&finding.mechanism, finding.status) {
                return Err(ParseReportError::InvalidFindingTrace {
                    mechanism_id: finding.mechanism.id.clone(),
                });
            }
        }

        let report = Self::new(
            wire.tool,
            wire.artifact,
            wire.findings,
            wire.limitations,
            wire.assumptions,
        )
        .map_err(ParseReportError::InvalidConstruction)?;
        Ok(UntrustedReport { report })
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
        "reference.synthid_text" => 9,
        "anthropic.embedded_text_watermark" => 10,
        _ => 11,
    }
}

#[derive(Serialize)]
struct SemanticReport<'a> {
    schema_version: &'static str,
    tool: &'a ToolIdentity,
    artifact: SemanticArtifactIdentity<'a>,
    findings: &'a [Finding],
    limitations: &'a [String],
    assumptions: &'a [String],
}

#[derive(Serialize)]
struct SemanticArtifactIdentity<'a> {
    byte_length: u64,
    content_sha256: &'a Sha256Digest,
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
    InvalidFindingTrace { mechanism_id: String },
    InvalidConstruction(ReportConstructionError),
    UnsupportedSchemaVersion { found: String },
}

impl fmt::Display for ParseReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid report JSON: {error}"),
            Self::InvalidFindingTrace { mechanism_id } => write!(
                formatter,
                "finding trace does not match the frozen ontology for mechanism {mechanism_id:?}"
            ),
            Self::InvalidConstruction(error) => error.fmt(formatter),
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
            Self::InvalidFindingTrace { .. } | Self::UnsupportedSchemaVersion { .. } => None,
            Self::InvalidConstruction(error) => Some(error),
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
        .expect("test report has valid unique findings")
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
    fn human_safe_visibly_escapes_terminal_layout_and_bidi_controls() {
        let hostile = concat!(
            "prefix",
            "\u{1b}[31mCSI",
            "\u{1b}]8;;https://example.invalid\u{7}OSC8\u{1b}]8;;\u{7}",
            "\r\n\t\u{8}\u{b}\u{7f}\u{85}",
            "\u{61c}\u{200e}\u{200f}\u{2028}\u{2029}",
            "\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}",
            "\u{2066}\u{2067}\u{2068}\u{2069}",
            "suffix"
        );
        let rendered = human_safe(hostile);

        assert!(rendered.starts_with("prefix\\u{1b}[31mCSI"));
        assert!(rendered.contains("\\u{d}\\u{a}\\u{9}\\u{8}\\u{b}\\u{7f}\\u{85}"));
        for code in [
            "61c", "200e", "200f", "2028", "2029", "202a", "202b", "202c", "202d", "202e", "2066",
            "2067", "2068", "2069",
        ] {
            assert!(rendered.contains(&format!("\\u{{{code}}}")), "U+{code}");
        }
        assert!(!rendered.chars().any(is_forbidden_human_control));
    }

    #[test]
    fn human_safe_preserves_ordinary_and_long_text_without_hidden_truncation() {
        let ordinary = "ASCII café 界 😀";
        assert_eq!(human_safe(ordinary), ordinary);

        let long = "A".repeat(10_000);
        assert_eq!(human_safe(&long), long);
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
            )
            .expect("test finding is valid"),
            Finding::new(
                MechanismIdentity::new("a.mechanism", "1"),
                FindingStatus::Present,
                vec![Evidence::new("raw", "value")],
                vec![],
                vec!["known input".to_owned()],
            )
            .expect("test finding is valid"),
        ];
        let report = report(findings);
        let json = report.to_json().expect("report serializes");
        assert_eq!(json, report.to_json().expect("report serializes again"));
        assert!(json.starts_with(concat!(
            "{\"schema_version\":\"0.2\",",
            "\"tool\":{\"name\":\"scrub\",\"version\":\"0.1.0\"},",
            "\"artifact\":{\"path\":\"sample.txt\",\"byte_length\":3,",
            "\"content_sha256\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"},",
            "\"findings\":[{\"mechanism\":{\"id\":\"a.mechanism\",\"version\":\"1\"},",
            "\"status\":\"present\",\"trace\":"
        )));
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["schema_version"], "0.2");
        assert_eq!(value["findings"][0]["mechanism"]["id"], "a.mechanism");
        assert_eq!(value["findings"][1]["mechanism"]["id"], "z.mechanism");
        assert_eq!(value["findings"][1]["evidence"][0]["name"], "a");
    }

    #[test]
    fn report_round_trip_normalizes_input_order() {
        let baseline = report(vec![]);
        let mut value: serde_json::Value =
            serde_json::from_str(&baseline.to_json().expect("report serializes"))
                .expect("valid JSON");
        value["limitations"] = serde_json::json!(["z", "a"]);
        value["assumptions"] = serde_json::json!(["z", "a"]);
        let input = serde_json::to_string(&value).expect("value serializes");
        let parsed = Report::from_json(&input)
            .expect("report parses as untrusted input")
            .into_report();
        let canonical = parsed.to_json().expect("report serializes");
        let canonical_value: serde_json::Value =
            serde_json::from_str(&canonical).expect("canonical report is JSON");
        assert_eq!(
            canonical_value["limitations"],
            serde_json::json!(["a", "z"])
        );
        assert_eq!(
            canonical_value["assumptions"],
            serde_json::json!(["a", "z"])
        );
        assert_eq!(
            Report::from_json(&canonical)
                .expect("canonical report parses as untrusted input")
                .into_report(),
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
            "anthropic.embedded_text_watermark",
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
                .expect("test finding is valid")
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
                "anthropic.embedded_text_watermark",
            ]
        );
    }

    #[test]
    fn unsupported_schema_version_is_typed() {
        let json = report(vec![])
            .to_json()
            .expect("report serializes")
            .replace("\"schema_version\":\"0.2\"", "\"schema_version\":\"9\"");

        assert!(matches!(
            Report::from_json(&json),
            Err(ParseReportError::UnsupportedSchemaVersion { found }) if found == "9"
        ));
    }

    #[test]
    fn unsupported_schema_version_precedes_v02_body_validation() {
        let json = r#"{"schema_version":"9","not_valid_under_v02":true}"#;

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
        )
        .expect("test finding is valid");
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
            "\"schema_version\":\"0.2\"",
            "\"schema_version\":\"0.2\",\"future\":true",
        );
        assert!(matches!(
            Report::from_json(&unknown_field),
            Err(ParseReportError::InvalidJson(_))
        ));
    }

    #[test]
    fn semantic_projection_excludes_display_path_and_is_locally_repeatable() {
        let first = report(vec![]);
        let second = Report::new(
            first.tool().clone(),
            ArtifactIdentity::new("elsewhere\\renamed.txt", 3, digest()),
            vec![],
            first.limitations().to_vec(),
            first.assumptions().to_vec(),
        )
        .expect("test report has valid unique findings");
        let first_bytes = first.canonical_report_bytes().expect("semantic bytes");
        assert_eq!(
            first_bytes,
            first
                .canonical_report_bytes()
                .expect("repeat semantic bytes")
        );
        assert_eq!(
            first_bytes,
            second.canonical_report_bytes().expect("semantic bytes")
        );
        assert!(
            !String::from_utf8(first_bytes)
                .expect("semantic JSON is UTF-8")
                .contains("sample.txt")
        );
    }

    #[test]
    fn claude_unknown_trace_forbids_provider_substitution_and_negative_inference() {
        let finding = Finding::new(
            MechanismIdentity::new("anthropic.embedded_text_watermark", "provider-doc"),
            FindingStatus::Unknown,
            vec![],
            vec![],
            vec![],
        )
        .expect("test finding is valid");
        assert_eq!(finding.status(), FindingStatus::Unknown);
        assert_eq!(
            finding
                .trace()
                .authority()
                .related_reference()
                .unwrap()
                .mechanism_id(),
            "reference.synthid_text"
        );
        assert!(
            finding
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeProviderParity)
        );
        assert!(
            finding
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeWatermarkAbsent)
        );
        assert!(
            !finding
                .trace()
                .supports()
                .contains(&InferenceId::ClaudeWatermarkPresent)
        );
    }

    #[test]
    fn anthropic_provider_states_require_a_runnable_provider_contract() {
        for status in [FindingStatus::Present, FindingStatus::Absent] {
            assert!(matches!(
                Finding::new(
                    MechanismIdentity::new("anthropic.embedded_text_watermark", "provider-doc"),
                    status,
                    vec![],
                    vec![],
                    vec![],
                ),
                Err(ReportConstructionError::InvalidMechanismStatus { .. })
            ));
        }
        assert!(
            Finding::new(
                MechanismIdentity::new("anthropic.embedded_text_watermark", "provider-doc"),
                FindingStatus::Unknown,
                vec![],
                vec![],
                vec![],
            )
            .is_ok()
        );
    }

    #[test]
    fn report_rejects_duplicate_mechanism_id_even_when_status_differs() {
        let make = |status| {
            Finding::new(
                MechanismIdentity::new("unicode.bidi_control", "1"),
                status,
                vec![],
                vec![],
                vec![],
            )
            .expect("valid finding")
        };
        assert!(matches!(
            Report::new(
                ToolIdentity::new("scrub", "0.1.0"),
                ArtifactIdentity::new("sample.txt", 3, digest()),
                vec![make(FindingStatus::Absent), make(FindingStatus::Present)],
                vec![], vec![],
            ),
            Err(ReportConstructionError::DuplicateMechanism { mechanism_id })
                if mechanism_id == "unicode.bidi_control"
        ));
    }

    #[test]
    fn imported_json_is_explicitly_untrusted() {
        let baseline = report(vec![]).to_json().expect("report serializes");
        let imported = Report::from_json(&baseline).expect("schema-valid report imports");
        assert_eq!(imported.as_report(), &report(vec![]));
        let mut value: serde_json::Value = serde_json::from_str(&baseline).expect("JSON");
        let finding = Finding::new(
            MechanismIdentity::new("unicode.bidi_control", "1"),
            FindingStatus::Absent,
            vec![],
            vec![],
            vec![],
        )
        .expect("valid finding");
        let encoded: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&finding).expect("finding serializes"))
                .expect("finding JSON");
        value["findings"] = serde_json::json!([encoded.clone(), encoded]);
        let duplicate = serde_json::to_string(&value).expect("JSON serializes");
        assert!(matches!(
            Report::from_json(&duplicate),
            Err(ParseReportError::InvalidConstruction(
                ReportConstructionError::DuplicateMechanism { .. }
            ))
        ));
    }

    #[test]
    fn public_synthid_result_cannot_become_claude_provider_evidence() {
        let finding = Finding::new(
            MechanismIdentity::new("reference.synthid_text", "pinned-reference"),
            FindingStatus::Present,
            vec![Evidence::new("weighted_mean", "0.75")],
            vec![],
            vec![],
        )
        .expect("test finding is valid");
        assert_eq!(
            finding.trace().authority().detector(),
            AuthorityClass::PublicReference
        );
        assert!(
            finding
                .trace()
                .supports()
                .contains(&InferenceId::PublicReferenceWatermarkPresent)
        );
        assert!(
            finding
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeProviderParity)
        );
        assert!(
            finding
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeWatermarkPresent)
        );
        assert_ne!(
            finding.mechanism().id(),
            "anthropic.embedded_text_watermark"
        );
    }

    #[test]
    fn c2pa_trace_preserves_presence_validity_binding_and_trust_ladder() {
        let make = |id| {
            Finding::new(
                MechanismIdentity::new(id, "2.4"),
                FindingStatus::Present,
                vec![],
                vec![],
                vec![],
            )
            .expect("test finding is valid")
        };
        let store = make("c2pa.manifest_store");
        assert!(
            store
                .trace()
                .supports()
                .contains(&InferenceId::C2paManifestStorePresent)
        );
        assert!(
            store
                .trace()
                .does_not_support()
                .contains(&InferenceId::C2paClaimValidity)
        );

        let validation = make("c2pa.manifest_validation");
        assert!(
            validation
                .trace()
                .supports()
                .contains(&InferenceId::C2paClaimValid)
        );
        assert!(
            validation
                .trace()
                .does_not_support()
                .contains(&InferenceId::C2paHardBinding)
        );

        let binding = make("c2pa.hard_binding");
        assert!(
            binding
                .trace()
                .supports()
                .contains(&InferenceId::C2paHardBindingValid)
        );
        assert!(
            binding
                .trace()
                .does_not_support()
                .contains(&InferenceId::C2paCredentialTrust)
        );

        let trust = make("c2pa.credential_trust");
        assert!(
            trust
                .trace()
                .supports()
                .contains(&InferenceId::C2paCredentialTrusted)
        );
        assert!(
            trust
                .trace()
                .does_not_support()
                .contains(&InferenceId::Authorship)
        );
    }

    #[test]
    fn decoding_rejects_a_trace_that_upgrades_provider_authority() {
        let finding = Finding::new(
            MechanismIdentity::new("anthropic.embedded_text_watermark", "provider-doc"),
            FindingStatus::Unknown,
            vec![],
            vec![],
            vec![],
        )
        .expect("test finding is valid");
        let json = report(vec![finding])
            .to_json()
            .expect("report serializes")
            .replace(
                "\"detector\":\"UNDOCUMENTED_PROVIDER\"",
                "\"detector\":\"PUBLIC_REFERENCE\"",
            );
        assert!(matches!(
            Report::from_json(&json),
            Err(ParseReportError::InvalidFindingTrace { mechanism_id })
                if mechanism_id == "anthropic.embedded_text_watermark"
        ));
    }
}
