use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use scrub::{
    C2PA_CREDENTIAL_TRUST_ID, C2PA_HARD_BINDING_ID, C2PA_MANIFEST_STORE_ID,
    C2PA_MANIFEST_VALIDATION_ID, C2PA_TEXT_WRAPPER_ID, PROVIDER_WATERMARK_ID,
    UNICODE_BIDI_CONTROL_ID, UNICODE_DEFAULT_IGNORABLE_ID, UNICODE_NFC_DIFFERENCE_ID,
    UNICODE_NFKC_DIFFERENCE_ID, inspect_file,
};
use scrub_report::{
    AuthorityClass, Finding, FindingStatus, InferenceId, ObservationKind, ReferenceRelationship,
    Report, VerifierAvailability, human_safe,
};

const USAGE: &str = "Usage: scrub inspect <path> [--explain] [--json]";

fn main() -> ExitCode {
    let command = match parse_args(env::args_os().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            write_diagnostic(&format_args!("error: {error}\n{USAGE}"));
            return ExitCode::from(2);
        }
    };

    let report = match inspect_file(&command.path) {
        Ok(report) => report,
        Err(error) => {
            write_diagnostic(&format_args!("error: {error}"));
            return ExitCode::from(1);
        }
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let result = match command.output {
        Output::Human if command.explain => write_explain(&mut stdout, &report),
        Output::Human => write_human(&mut stdout, &report),
        Output::Json => write_json(&mut stdout, &report),
    };
    if let Err(error) = result {
        write_diagnostic(&format_args!("error: {error}"));
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Output {
    Human,
    Json,
}

#[derive(Debug, Eq, PartialEq)]
struct InspectCommand {
    path: PathBuf,
    output: Output,
    explain: bool,
}

fn parse_args(arguments: impl IntoIterator<Item = OsString>) -> Result<InspectCommand, CliError> {
    let mut arguments = arguments.into_iter();
    match arguments.next() {
        Some(command) if command == "inspect" => {}
        Some(command) => return Err(CliError::UnknownCommand(command)),
        None => return Err(CliError::MissingCommand),
    }

    let mut path = None;
    let mut output = Output::Human;
    let mut explain = false;
    let mut options = true;
    for argument in arguments {
        if options && argument == "--" {
            options = false;
        } else if options && argument == "--json" {
            if output == Output::Json {
                return Err(CliError::DuplicateJsonOption);
            }
            output = Output::Json;
        } else if options && argument == "--explain" {
            if explain {
                return Err(CliError::DuplicateExplainOption);
            }
            explain = true;
        } else if options && argument.to_string_lossy().starts_with('-') {
            return Err(CliError::UnknownOption(argument));
        } else if path.replace(PathBuf::from(argument)).is_some() {
            return Err(CliError::TooManyPaths);
        }
    }

    let path = path.ok_or(CliError::MissingPath)?;
    Ok(InspectCommand {
        path,
        output,
        explain,
    })
}

#[derive(Debug, Eq, PartialEq)]
enum CliError {
    MissingCommand,
    UnknownCommand(OsString),
    MissingPath,
    TooManyPaths,
    UnknownOption(OsString),
    DuplicateJsonOption,
    DuplicateExplainOption,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("missing command"),
            Self::UnknownCommand(command) => {
                write!(
                    formatter,
                    "unknown command: {}",
                    human_safe(&command.to_string_lossy())
                )
            }
            Self::MissingPath => formatter.write_str("inspect requires one file path"),
            Self::TooManyPaths => formatter.write_str("inspect accepts only one file path"),
            Self::UnknownOption(option) => {
                write!(
                    formatter,
                    "unknown option: {}",
                    human_safe(&option.to_string_lossy())
                )
            }
            Self::DuplicateJsonOption => formatter.write_str("--json may be specified only once"),
            Self::DuplicateExplainOption => {
                formatter.write_str("--explain may be specified only once")
            }
        }
    }
}

impl Error for CliError {}

fn write_human(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    writeln!(writer, "scrub inspect")?;
    writeln!(writer)?;
    writeln!(writer, "Artifact")?;
    writeln!(writer, "  name    {}", human_safe(report.artifact().path()))?;
    writeln!(writer, "  bytes   {}", report.artifact().byte_length())?;
    writeln!(writer, "  sha256  {}", report.artifact().content_sha256())?;

    let unicode: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.mechanism().id().starts_with("unicode."))
        .collect();
    writeln!(writer)?;
    writeln!(writer, "Evidence")?;
    let notable_unicode: Vec<_> = unicode
        .iter()
        .copied()
        .filter(|finding| finding.status() != FindingStatus::Absent)
        .collect();
    if notable_unicode.is_empty() {
        writeln!(
            writer,
            "  Unicode  ABSENT  no listed property or normalization difference observed"
        )?;
    } else {
        for finding in notable_unicode {
            write_finding_summary(writer, "Unicode", finding)?;
        }
    }

    let c2pa: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.mechanism().id().starts_with("c2pa."))
        .collect();
    let notable_c2pa: Vec<_> = c2pa
        .iter()
        .copied()
        .filter(|finding| {
            finding.status() != FindingStatus::NotApplicable
                && !(finding.status() == FindingStatus::Absent
                    && finding.mechanism().id() != C2PA_MANIFEST_STORE_ID)
        })
        .collect();
    if notable_c2pa.is_empty() {
        if let Some(wrapper) = c2pa
            .iter()
            .copied()
            .find(|finding| finding.mechanism().id() == C2PA_TEXT_WRAPPER_ID)
            .filter(|finding| finding.status() == FindingStatus::Absent)
        {
            write_finding_summary(writer, "C2PA", wrapper)?;
        } else {
            writeln!(writer, "  C2PA     NOT_APPLICABLE")?;
        }
    } else {
        for finding in notable_c2pa {
            write_finding_summary(writer, "C2PA", finding)?;
        }
    }

    if let Some(provider) = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == PROVIDER_WATERMARK_ID)
    {
        write_finding_summary(writer, "Claude", provider)?;
        if provider.status() == FindingStatus::Unknown {
            let verifier = provider.trace().verifier();
            writeln!(
                writer,
                "             verifier {} ({})",
                verifier.id(),
                verifier_availability_label(verifier.availability())
            )?;
            if let Some(reference) = provider.trace().authority().related_reference() {
                writeln!(
                    writer,
                    "             related reference {} ({})",
                    reference.mechanism_id(),
                    reference_relationship_label(reference.relationship())
                )?;
            }
            if provider
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeProviderParity)
            {
                writeln!(
                    writer,
                    "             does not support {}",
                    inference_label(InferenceId::ClaudeProviderParity)
                )?;
            }
        }
    }

    writeln!(writer)?;
    writeln!(writer, "Boundary")?;
    writeln!(
        writer,
        "  UNKNOWN and UNSUPPORTED are not negative findings; this report does not establish authorship or that the artifact is clean."
    )?;
    writeln!(
        writer,
        "  Use --explain for the authority and inference trace."
    )?;
    Ok(())
}

fn write_finding_summary(
    writer: &mut impl Write,
    group: &str,
    finding: &Finding,
) -> Result<(), OutputError> {
    writeln!(
        writer,
        "  {group:<9}{:<13}{}",
        finding.status().as_str().to_ascii_uppercase(),
        mechanism_display_name(finding.mechanism().id())
    )?;
    if finding.status() == FindingStatus::Present {
        for evidence in finding.evidence().iter().take(2) {
            writeln!(
                writer,
                "             {}={}",
                human_safe(evidence.name()),
                human_safe(evidence.value())
            )?;
        }
    }
    Ok(())
}

fn write_explain(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    write_human(writer, report)?;
    writeln!(writer)?;
    writeln!(writer, "Evidence trace")?;
    for finding in report.findings() {
        writeln!(writer)?;
        writeln!(
            writer,
            "{} [{}]",
            mechanism_display_name(finding.mechanism().id()),
            finding.mechanism().id()
        )?;
        writeln!(writer, "  status        {}", finding.status())?;
        writeln!(
            writer,
            "  observation   {}",
            observation_label(finding.trace().observation())
        )?;
        if finding.evidence().is_empty() {
            writeln!(writer, "  evidence      none retained")?;
        } else {
            for evidence in finding.evidence() {
                writeln!(
                    writer,
                    "  evidence      {}={}",
                    human_safe(evidence.name()),
                    human_safe(evidence.value())
                )?;
            }
        }
        let verifier = finding.trace().verifier();
        writeln!(
            writer,
            "  verifier      {} {} ({})",
            verifier.id(),
            verifier.version(),
            verifier_availability_label(verifier.availability())
        )?;
        let authority = finding.trace().authority();
        writeln!(
            writer,
            "  authority     mechanism={}, implementation={}, detector={}",
            authority_label(authority.mechanism()),
            authority_label(authority.implementation()),
            authority_label(authority.detector())
        )?;
        writeln!(
            writer,
            "  sources       {}",
            authority.source_ids().join(", ")
        )?;
        if let Some(configuration) = finding.trace().configuration_identity() {
            writeln!(writer, "  configuration {configuration}")?;
        }
        if let Some(reference) = authority.related_reference() {
            writeln!(
                writer,
                "  related       {} ({})",
                reference.mechanism_id(),
                reference_relationship_label(reference.relationship())
            )?;
        }
        write_inferences(writer, "supports", finding.trace().supports())?;
        write_inferences(
            writer,
            "does not support",
            finding.trace().does_not_support(),
        )?;
        for limitation in finding.limitations() {
            writeln!(writer, "  limitation    {}", human_safe(limitation))?;
        }
        writeln!(
            writer,
            "  reproduce     {}",
            finding.trace().reproduce().command().join(" ")
        )?;
    }
    Ok(())
}

fn write_inferences(
    writer: &mut impl Write,
    label: &str,
    inferences: &[InferenceId],
) -> Result<(), OutputError> {
    if inferences.is_empty() {
        writeln!(writer, "  {label:<13}none")?;
    } else {
        for (index, inference) in inferences.iter().enumerate() {
            writeln!(
                writer,
                "  {:<13}{}",
                if index == 0 { label } else { "" },
                inference_label(*inference)
            )?;
        }
    }
    Ok(())
}

fn mechanism_display_name(mechanism_id: &str) -> &str {
    match mechanism_id {
        UNICODE_BIDI_CONTROL_ID => "Bidi_Control",
        UNICODE_DEFAULT_IGNORABLE_ID => "Default_Ignorable_Code_Point",
        UNICODE_NFC_DIFFERENCE_ID => "NFC difference",
        UNICODE_NFKC_DIFFERENCE_ID => "NFKC difference",
        C2PA_TEXT_WRAPPER_ID => "text manifest wrapper",
        C2PA_MANIFEST_STORE_ID => "manifest store",
        C2PA_MANIFEST_VALIDATION_ID => "manifest validation",
        C2PA_HARD_BINDING_ID => "hard binding",
        C2PA_CREDENTIAL_TRUST_ID => "credential trust",
        PROVIDER_WATERMARK_ID => "embedded text watermark",
        _ => mechanism_id,
    }
}

const fn observation_label(observation: ObservationKind) -> &'static str {
    match observation {
        ObservationKind::UnicodePropertyMembership => "Unicode property membership",
        ObservationKind::UnicodeNormalizationDifference => "Unicode normalization difference",
        ObservationKind::C2paTextManifestWrapper => "C2PA Appendix A.8 carrier",
        ObservationKind::C2paManifestStore => "C2PA manifest store",
        ObservationKind::C2paManifestValidation => "C2PA claim validation",
        ObservationKind::C2paHardBinding => "C2PA hard binding",
        ObservationKind::C2paCredentialTrust => "C2PA credential trust",
        ObservationKind::StatisticalWatermarkDecision => "statistical watermark decision",
        ObservationKind::ProviderTextWatermark => "provider text watermark",
        ObservationKind::Unclassified => "unclassified observation",
    }
}

const fn authority_label(authority: AuthorityClass) -> &'static str {
    match authority {
        AuthorityClass::Standard => "STANDARD",
        AuthorityClass::PublicReference => "PUBLIC_REFERENCE",
        AuthorityClass::PublicMechanismPrivateKey => "PUBLIC_MECHANISM_PRIVATE_KEY",
        AuthorityClass::UndocumentedProvider => "UNDOCUMENTED_PROVIDER",
        AuthorityClass::ProjectImplementation => "PROJECT_IMPLEMENTATION",
        AuthorityClass::Unspecified => "UNSPECIFIED",
    }
}

const fn verifier_availability_label(availability: VerifierAvailability) -> &'static str {
    match availability {
        VerifierAvailability::Available => "available",
        VerifierAvailability::Unavailable => "unavailable in checked authority snapshot",
    }
}

const fn reference_relationship_label(relationship: ReferenceRelationship) -> &'static str {
    match relationship {
        ReferenceRelationship::RelatedFamilyNotDeploymentParity => {
            "related family; not deployment parity"
        }
    }
}

const fn inference_label(inference: InferenceId) -> &'static str {
    match inference {
        InferenceId::UnicodePropertyPresent => "unicode_property_present",
        InferenceId::UnicodePropertyAbsentWithinScope => "unicode_property_absent_within_scope",
        InferenceId::UnicodeNormalizationDifferencePresent => {
            "unicode_normalization_difference_present"
        }
        InferenceId::UnicodeNormalizationDifferenceAbsentWithinScope => {
            "unicode_normalization_difference_absent_within_scope"
        }
        InferenceId::C2paCarrierPresent => "c2pa_carrier_present",
        InferenceId::C2paCarrierAbsentWithinScope => "c2pa_carrier_absent_within_scope",
        InferenceId::C2paManifestStorePresent => "c2pa_manifest_store_present",
        InferenceId::C2paManifestStoreAbsentWithinScope => {
            "c2pa_manifest_store_absent_within_scope"
        }
        InferenceId::C2paClaimValid => "c2pa_claim_valid",
        InferenceId::C2paHardBindingValid => "c2pa_hard_binding_valid",
        InferenceId::C2paCredentialTrusted => "c2pa_credential_trusted",
        InferenceId::ProviderMechanismFamilyDisclosed => "provider_mechanism_family_disclosed",
        InferenceId::ProviderDetectorUnavailable => "provider_detector_unavailable",
        InferenceId::PublicReferenceWatermarkPresent => "public_reference_watermark_present",
        InferenceId::PublicReferenceWatermarkAbsentWithinConfiguration => {
            "public_reference_watermark_absent_within_configuration"
        }
        InferenceId::ArtifactClean => "artifact_clean",
        InferenceId::HumanAuthorship => "human_authorship",
        InferenceId::AiAuthorship => "ai_authorship",
        InferenceId::MaliciousIntent => "malicious_intent",
        InferenceId::WatermarkPresent => "watermark_present",
        InferenceId::WatermarkAbsent => "watermark_absent",
        InferenceId::C2paClaimValidity => "c2pa_claim_validity",
        InferenceId::C2paHardBinding => "c2pa_hard_binding",
        InferenceId::C2paCredentialTrust => "c2pa_credential_trust",
        InferenceId::Authorship => "authorship",
        InferenceId::Truthfulness => "truthfulness",
        InferenceId::ClaudeWatermarkPresent => "claude_watermark_present",
        InferenceId::ClaudeWatermarkAbsent => "claude_watermark_absent",
        InferenceId::ClaudeProviderParity => "claude_provider_parity",
    }
}

fn write_json(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    let json = report
        .to_json()
        .map_err(|error| OutputError::Serialization(error.to_string()))?;
    writeln!(writer, "{json}")?;
    Ok(())
}

#[derive(Debug)]
enum OutputError {
    Io(io::Error),
    Serialization(String),
}

impl fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not write output: {error}"),
            Self::Serialization(error) => write!(formatter, "could not serialize report: {error}"),
        }
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Serialization(_) => None,
        }
    }
}

impl From<io::Error> for OutputError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn write_diagnostic(arguments: &fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let _ = writeln!(stderr, "{arguments}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrub_report::{ArtifactIdentity, Sha256Digest, ToolIdentity};

    #[test]
    fn parses_json_before_or_after_path() {
        for arguments in [
            ["inspect", "artifact.txt", "--json"],
            ["inspect", "--json", "artifact.txt"],
        ] {
            let parsed = parse_args(arguments.map(OsString::from)).expect("arguments are valid");
            assert_eq!(parsed.path, PathBuf::from("artifact.txt"));
            assert_eq!(parsed.output, Output::Json);
        }
    }

    #[test]
    fn double_dash_allows_a_dash_prefixed_path() {
        let parsed = parse_args(["inspect", "--", "--artifact"].map(OsString::from))
            .expect("arguments are valid");
        assert_eq!(parsed.path, PathBuf::from("--artifact"));
        assert_eq!(parsed.output, Output::Human);
    }

    #[test]
    fn usage_diagnostics_escape_untrusted_controls_and_bidi() {
        let error = CliError::UnknownOption(OsString::from("--x\u{1b}]8;;url\u{7}\u{202e}"));
        let rendered = error.to_string();
        assert_eq!(rendered, "unknown option: --x\\u{1b}]8;;url\\u{7}\\u{202e}");
        assert!(
            !rendered
                .chars()
                .any(scrub_report::is_forbidden_human_control)
        );
    }

    #[test]
    fn complete_human_renderer_escapes_a_hostile_display_name() {
        let report = Report::new(
            ToolIdentity::new("scrub", "test"),
            ArtifactIdentity::new(
                "victim\u{1b}[31m\u{1b}]8;;https://example.invalid\u{7}\r\n\t\u{8}\u{202e}.txt",
                0,
                Sha256Digest::from_bytes([0; 32]),
            ),
            vec![],
            vec![],
            vec![],
        )
        .expect("synthetic report is valid");
        let mut output = Vec::new();
        write_human(&mut output, &report).expect("human rendering succeeds");
        let output = String::from_utf8(output).expect("human output is UTF-8");

        assert!(output.contains(
            "victim\\u{1b}[31m\\u{1b}]8;;https://example.invalid\\u{7}\\u{d}\\u{a}\\u{9}\\u{8}\\u{202e}.txt"
        ));
        assert!(
            !output
                .chars()
                .any(|scalar| scalar != '\n' && scrub_report::is_forbidden_human_control(scalar))
        );
    }
}
