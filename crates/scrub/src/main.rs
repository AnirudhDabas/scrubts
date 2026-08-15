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
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--help" {
        return write_identity_output(&format_args!(
            "{USAGE}\n\nInspect one local artifact without network access."
        ));
    }
    if arguments.len() == 1 && arguments[0] == "--version" {
        return write_identity_output(&format_args!("scrub {}", env!("CARGO_PKG_VERSION")));
    }

    let command = match parse_args(arguments) {
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

fn write_identity_output(arguments: &fmt::Arguments<'_>) -> ExitCode {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if let Err(error) = writeln!(stdout, "{arguments}") {
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
    writeln!(
        writer,
        "  size    {} bytes",
        report.artifact().byte_length()
    )?;
    writeln!(writer, "  sha256  {}", report.artifact().content_sha256())?;

    let unicode: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.mechanism().id().starts_with("unicode."))
        .collect();
    writeln!(writer)?;
    writeln!(writer, "Observations")?;
    writeln!(writer)?;
    writeln!(writer, "  Unicode")?;
    let notable_unicode: Vec<_> = unicode
        .iter()
        .copied()
        .filter(|finding| finding.status() != FindingStatus::Absent)
        .collect();
    let has_present_unicode = notable_unicode
        .iter()
        .any(|finding| finding.status() == FindingStatus::Present);
    if notable_unicode.is_empty() {
        writeln!(
            writer,
            "    ABSENT          no listed property or normalization difference observed"
        )?;
    } else {
        for &finding in &notable_unicode {
            write_finding_summary(writer, finding)?;
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
    writeln!(writer)?;
    writeln!(writer, "  C2PA")?;
    if notable_c2pa.is_empty() {
        if let Some(wrapper) = c2pa
            .iter()
            .copied()
            .find(|finding| finding.mechanism().id() == C2PA_TEXT_WRAPPER_ID)
            .filter(|finding| finding.status() == FindingStatus::Absent)
        {
            write_finding_summary(writer, wrapper)?;
        } else {
            writeln!(writer, "    NOT_APPLICABLE")?;
        }
    } else {
        for finding in notable_c2pa {
            write_finding_summary(writer, finding)?;
        }
    }

    if let Some(provider) = report
        .findings()
        .iter()
        .find(|finding| finding.mechanism().id() == PROVIDER_WATERMARK_ID)
    {
        writeln!(writer)?;
        writeln!(writer, "  Claude")?;
        write_finding_summary(writer, provider)?;
        if provider.status() == FindingStatus::Unknown {
            let verifier = provider.trace().verifier();
            write_summary_field(writer, "verifier", &human_safe(verifier.id()))?;
            write_summary_continuation(
                writer,
                verifier_availability_label(verifier.availability()),
            )?;
            if let Some(reference) = provider.trace().authority().related_reference() {
                write_summary_field(writer, "reference", &human_safe(reference.mechanism_id()))?;
                write_summary_continuation(
                    writer,
                    reference_relationship_label(reference.relationship()),
                )?;
            }
            let supports_family = provider
                .trace()
                .supports()
                .contains(&InferenceId::ProviderMechanismFamilyDisclosed);
            let supports_unavailable = provider
                .trace()
                .supports()
                .contains(&InferenceId::ProviderDetectorUnavailable);
            if supports_family && supports_unavailable {
                write_summary_field(
                    writer,
                    "supports",
                    "mechanism family disclosed; provider detector unavailable",
                )?;
            }
            let does_not_support_presence = provider
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeWatermarkPresent);
            let does_not_support_absence = provider
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeWatermarkAbsent);
            let does_not_support_parity = provider
                .trace()
                .does_not_support()
                .contains(&InferenceId::ClaudeProviderParity);
            if does_not_support_presence && does_not_support_absence && does_not_support_parity {
                write_summary_field(
                    writer,
                    "does not support",
                    "Claude watermark presence/absence or provider parity",
                )?;
            }
        }
    }

    writeln!(writer)?;
    writeln!(writer, "Interpretation")?;
    if has_present_unicode {
        writeln!(
            writer,
            "  A Unicode PRESENT finding supports only its reported Unicode observation."
        )?;
        writeln!(
            writer,
            "  It does not establish an Anthropic watermark, Claude involvement, or authorship."
        )?;
    }
    writeln!(writer, "  UNKNOWN != ABSENT / CLEAN.")?;
    writeln!(
        writer,
        "  No aggregate authorship or artifact-clean verdict is reported."
    )?;
    writeln!(
        writer,
        "  Use --explain for the complete evidence and authority chain."
    )?;
    Ok(())
}

fn write_finding_summary(writer: &mut impl Write, finding: &Finding) -> Result<(), OutputError> {
    writeln!(
        writer,
        "    {:<14}  {}",
        finding.status().as_str().to_ascii_uppercase(),
        human_safe(mechanism_display_name(finding.mechanism().id()))
    )?;
    if finding.status() == FindingStatus::Present {
        for evidence in finding.evidence().iter().take(2) {
            if evidence.name() == "locations"
                && let Some(locations) = parse_unicode_locations(evidence.value())
                && !locations.is_empty()
            {
                for location in locations {
                    let abbreviation = location
                        .abbreviation
                        .map(|value| format!(" ({})", human_safe(value)))
                        .unwrap_or_default();
                    write_summary_field(
                        writer,
                        "evidence",
                        &format!(
                            "{}{} at byte offset {}, scalar offset {}",
                            human_safe(location.code_point),
                            abbreviation,
                            location.byte_offset,
                            location.scalar_offset
                        ),
                    )?;
                }
            } else if evidence.name() == "locations_truncated" && evidence.value() == "false" {
                continue;
            } else {
                write_summary_field(
                    writer,
                    &human_safe(evidence.name()),
                    &human_safe(evidence.value()),
                )?;
            }
        }
    }
    Ok(())
}

fn write_summary_field(
    writer: &mut impl Write,
    label: &str,
    value: &str,
) -> Result<(), OutputError> {
    writeln!(writer, "      {label:<16}  {value}")?;
    Ok(())
}

fn write_summary_continuation(writer: &mut impl Write, value: &str) -> Result<(), OutputError> {
    writeln!(writer, "                        {value}")?;
    Ok(())
}

#[derive(Clone, Copy)]
struct HumanUnicodeLocation<'a> {
    code_point: &'a str,
    abbreviation: Option<&'a str>,
    byte_offset: &'a str,
    scalar_offset: &'a str,
}

fn parse_unicode_locations(value: &str) -> Option<Vec<HumanUnicodeLocation<'_>>> {
    if value == "[]" {
        return Some(Vec::new());
    }
    let inner = value.strip_prefix("[{")?.strip_suffix("}]")?;
    inner.split("},{").map(parse_unicode_location).collect()
}

fn parse_unicode_location(value: &str) -> Option<HumanUnicodeLocation<'_>> {
    let value = value.strip_prefix("\"code_point\":\"")?;
    let (code_point, value) = value.split_once('"')?;
    if code_point.len() <= 2
        || !code_point.starts_with("U+")
        || !code_point[2..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }

    let (abbreviation, value) = if let Some(value) = value.strip_prefix(",\"abbreviation\":\"") {
        let (abbreviation, value) = value.split_once('"')?;
        if abbreviation.is_empty()
            || !abbreviation
                .chars()
                .all(|character| character.is_ascii_uppercase())
        {
            return None;
        }
        (Some(abbreviation), value)
    } else {
        (None, value)
    };

    let value = value.strip_prefix(",\"byte_offset\":")?;
    let (byte_offset, scalar_offset) = value.split_once(",\"scalar_offset\":")?;
    if byte_offset.is_empty()
        || scalar_offset.is_empty()
        || !byte_offset
            .chars()
            .all(|character| character.is_ascii_digit())
        || !scalar_offset
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return None;
    }

    Some(HumanUnicodeLocation {
        code_point,
        abbreviation,
        byte_offset,
        scalar_offset,
    })
}

fn write_explain(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    write_human(writer, report)?;
    writeln!(writer)?;
    writeln!(writer, "Complete trace")?;
    for finding in report.findings() {
        writeln!(writer)?;
        writeln!(
            writer,
            "{}",
            human_safe(mechanism_display_name(finding.mechanism().id()))
        )?;
        write_trace_field(writer, "mechanism", &human_safe(finding.mechanism().id()))?;
        write_trace_field(
            writer,
            "status",
            &finding.status().as_str().to_ascii_uppercase(),
        )?;
        write_trace_field(
            writer,
            "observation",
            observation_label(finding.trace().observation()),
        )?;
        if finding.evidence().is_empty() {
            write_trace_field(writer, "evidence", "none retained")?;
        } else {
            for evidence in finding.evidence() {
                write_trace_field(
                    writer,
                    "evidence",
                    &format!(
                        "{}={}",
                        human_safe(evidence.name()),
                        human_safe(evidence.value())
                    ),
                )?;
            }
        }
        let verifier = finding.trace().verifier();
        write_trace_field(
            writer,
            "verifier",
            &format!(
                "{} {}",
                human_safe(verifier.id()),
                human_safe(verifier.version())
            ),
        )?;
        write_trace_field(
            writer,
            "availability",
            verifier_availability_label(verifier.availability()),
        )?;
        let authority = finding.trace().authority();
        write_trace_field(
            writer,
            "authority",
            &format!("mechanism {}", authority_label(authority.mechanism())),
        )?;
        write_trace_continuation(
            writer,
            &format!(
                "implementation {}",
                authority_label(authority.implementation())
            ),
        )?;
        write_trace_continuation(
            writer,
            &format!("detector {}", authority_label(authority.detector())),
        )?;
        let sources: Vec<_> = authority
            .source_ids()
            .iter()
            .map(|source| human_safe(source))
            .collect();
        write_trace_values(writer, "sources", &sources)?;
        if let Some(configuration) = finding.trace().configuration_identity() {
            write_trace_field(writer, "configuration", &human_safe(configuration))?;
        }
        if let Some(reference) = authority.related_reference() {
            write_trace_field(
                writer,
                "related reference",
                &human_safe(reference.mechanism_id()),
            )?;
            write_trace_continuation(
                writer,
                reference_relationship_label(reference.relationship()),
            )?;
        }
        write_inferences(writer, "supports", finding.trace().supports())?;
        write_inferences(
            writer,
            "does not support",
            finding.trace().does_not_support(),
        )?;
        for limitation in finding.limitations() {
            write_wrapped_trace_field(writer, "limitation", &human_safe(limitation))?;
        }
        write_trace_field(
            writer,
            "reproduce",
            &human_safe(&finding.trace().reproduce().command().join(" ")),
        )?;
    }
    Ok(())
}

fn write_trace_field(writer: &mut impl Write, label: &str, value: &str) -> Result<(), OutputError> {
    writeln!(writer, "  {label:<18}{value}")?;
    Ok(())
}

fn write_trace_continuation(writer: &mut impl Write, value: &str) -> Result<(), OutputError> {
    writeln!(writer, "                    {value}")?;
    Ok(())
}

fn write_trace_values(
    writer: &mut impl Write,
    label: &str,
    values: &[String],
) -> Result<(), OutputError> {
    if values.is_empty() {
        write_trace_field(writer, label, "none")?;
    } else {
        for (index, value) in values.iter().enumerate() {
            if index == 0 {
                write_trace_field(writer, label, value)?;
            } else {
                write_trace_continuation(writer, value)?;
            }
        }
    }
    Ok(())
}

fn write_wrapped_trace_field(
    writer: &mut impl Write,
    label: &str,
    value: &str,
) -> Result<(), OutputError> {
    const VALUE_WIDTH: usize = 78;

    let mut line = String::new();
    let mut first = true;
    for word in value.split_whitespace() {
        let next_width =
            line.chars().count() + usize::from(!line.is_empty()) + word.chars().count();
        if !line.is_empty() && next_width > VALUE_WIDTH {
            if first {
                write_trace_field(writer, label, &line)?;
                first = false;
            } else {
                write_trace_continuation(writer, &line)?;
            }
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if first {
        write_trace_field(writer, label, &line)?;
    } else if !line.is_empty() {
        write_trace_continuation(writer, &line)?;
    }
    Ok(())
}

fn write_inferences(
    writer: &mut impl Write,
    label: &str,
    inferences: &[InferenceId],
) -> Result<(), OutputError> {
    let values: Vec<_> = inferences
        .iter()
        .map(|inference| inference_label(*inference).to_owned())
        .collect();
    write_trace_values(writer, label, &values)
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
