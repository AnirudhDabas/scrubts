use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use scrub_report::{
    ArtifactIdentity, AuthorityClass, Finding, FindingStatus, InferenceId, ObservationKind,
    ReferenceRelationship, Report, Sha256Digest, ToolIdentity, VerifierAvailability,
};
use sha2::{Digest, Sha256};

mod c2pa_provenance;
mod provider_watermark;
mod unicode_bidi_control;
mod unicode_default_ignorable;
mod unicode_normalization;
mod utf8_stream;

const USAGE: &str = "Usage: scrub inspect <path> [--explain] [--json]";
const UNEVALUATED_MECHANISMS_LIMITATION: &str = "Inspection evaluates the listed Unicode and C2PA mechanisms and represents Claude provider-detector availability; confusables, sanitization, unrelated metadata, public-reference statistical detectors, and WaterLARP mechanisms are not evaluated.";

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
                    "unknown command: {:?}",
                    command.to_string_lossy()
                )
            }
            Self::MissingPath => formatter.write_str("inspect requires one file path"),
            Self::TooManyPaths => formatter.write_str("inspect accepts only one file path"),
            Self::UnknownOption(option) => {
                write!(formatter, "unknown option: {:?}", option.to_string_lossy())
            }
            Self::DuplicateJsonOption => formatter.write_str("--json may be specified only once"),
            Self::DuplicateExplainOption => {
                formatter.write_str("--explain may be specified only once")
            }
        }
    }
}

impl Error for CliError {}

fn inspect_file(path: &Path) -> Result<Report, InspectError> {
    let path_metadata = fs::metadata(path).map_err(|source| InspectError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    if !path_metadata.is_file() {
        return Err(InspectError::NotRegularFile(path.to_path_buf()));
    }

    let mut file = File::open(path).map_err(|source| InspectError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let metadata = file.metadata().map_err(|source| InspectError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(InspectError::NotRegularFile(path.to_path_buf()));
    }

    inspect_reader(path, &mut file)
}

fn inspect_reader(path: &Path, reader: &mut impl Read) -> Result<Report, InspectError> {
    let mut hasher = Sha256::new();
    let mut decoder = utf8_stream::Decoder::new();
    let mut dicp_inspection = unicode_default_ignorable::Inspection::new();
    let mut bidi_inspection = unicode_bidi_control::Inspection::new();
    let mut normalization_bytes = Vec::new();
    let mut normalization_buffer_failed = false;
    let mut c2pa_prefix = [0_u8; 8];
    let mut c2pa_prefix_len = 0_usize;
    let mut c2pa_binary: Option<(c2pa_provenance::BinaryFormat, Vec<u8>)> = None;
    let mut c2pa_buffer_failed = false;
    let mut byte_length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|source| InspectError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        if count == 0 {
            break;
        }
        if !c2pa_buffer_failed {
            if let Some((_, bytes)) = c2pa_binary.as_mut() {
                if bytes.try_reserve(count).is_err() {
                    c2pa_buffer_failed = true;
                    bytes.clear();
                } else {
                    bytes.extend_from_slice(&buffer[..count]);
                }
            } else if c2pa_prefix_len < c2pa_prefix.len() {
                let copied = (c2pa_prefix.len() - c2pa_prefix_len).min(count);
                c2pa_prefix[c2pa_prefix_len..c2pa_prefix_len + copied]
                    .copy_from_slice(&buffer[..copied]);
                c2pa_prefix_len += copied;
                if let Some(format) =
                    c2pa_provenance::detect_binary(&c2pa_prefix[..c2pa_prefix_len])
                {
                    let previous =
                        usize::try_from(byte_length).map_err(|_| InspectError::ArtifactTooLarge)?;
                    let required = previous
                        .checked_add(count)
                        .ok_or(InspectError::ArtifactTooLarge)?;
                    let mut bytes = Vec::new();
                    if bytes.try_reserve(required).is_err() {
                        c2pa_buffer_failed = true;
                    } else {
                        bytes.extend_from_slice(&c2pa_prefix[..previous]);
                        bytes.extend_from_slice(&buffer[..count]);
                        c2pa_binary = Some((format, bytes));
                    }
                }
            }
        }
        hasher.update(&buffer[..count]);
        decoder
            .push(&buffer[..count], |observation| {
                dicp_inspection.observe(observation).map_err(|_| ())?;
                bidi_inspection.observe(observation).map_err(|_| ())
            })
            .map_err(|_| InspectError::ArtifactTooLarge)?;
        if decoder.is_potentially_valid() && !normalization_buffer_failed {
            if normalization_bytes.try_reserve(count).is_err() {
                normalization_buffer_failed = true;
                normalization_bytes.clear();
            } else {
                normalization_bytes.extend_from_slice(&buffer[..count]);
            }
        } else if !decoder.is_potentially_valid() {
            normalization_bytes.clear();
        }
        let count = u64::try_from(count).map_err(|_| InspectError::ArtifactTooLarge)?;
        byte_length = byte_length
            .checked_add(count)
            .ok_or(InspectError::ArtifactTooLarge)?;
    }

    if normalization_buffer_failed {
        return Err(InspectError::NormalizationBufferAllocation);
    }
    if c2pa_buffer_failed {
        return Err(InspectError::C2paBufferAllocation);
    }

    let digest: [u8; 32] = hasher.finalize().into();
    let mut limitations = vec![UNEVALUATED_MECHANISMS_LIMITATION.to_owned()];
    let display_path = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_owned(),
        None => {
            limitations
                .push("The artifact display name was lossily converted to Unicode.".to_owned());
            path.file_name().map_or_else(
                || "<artifact>".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            )
        }
    };

    let valid_utf8 = decoder.finish().is_ok();
    let [nfc_finding, nfkc_finding] = if valid_utf8 {
        let input = std::str::from_utf8(&normalization_bytes)
            .map_err(|_| InspectError::NormalizationUtf8Invariant)?;
        unicode_normalization::valid_findings(input).map_err(InspectError::Normalization)?
    } else {
        unicode_normalization::invalid_findings()
    };
    let textual_artifact = c2pa_binary.is_none() && (valid_utf8 || is_plain_text_path(path));
    let c2pa_findings = if let Some((format, bytes)) = c2pa_binary {
        c2pa_provenance::binary_format_findings(format, &bytes).map_err(InspectError::C2pa)?
    } else if valid_utf8 {
        let input = std::str::from_utf8(&normalization_bytes)
            .map_err(|_| InspectError::NormalizationUtf8Invariant)?;
        c2pa_provenance::valid_text_findings(input).map_err(InspectError::C2pa)?
    } else if is_plain_text_path(path) {
        c2pa_provenance::malformed_text_findings()
    } else {
        c2pa_provenance::unsupported_findings()
    };
    let mut findings = vec![
        dicp_inspection.finish(valid_utf8),
        bidi_inspection.finish(valid_utf8),
        nfc_finding,
        nfkc_finding,
    ];
    findings.extend(c2pa_findings);
    findings.push(provider_watermark::finding(textual_artifact));
    Ok(Report::new(
        ToolIdentity::new("scrub", env!("CARGO_PKG_VERSION")),
        ArtifactIdentity::new(display_path, byte_length, Sha256Digest::from_bytes(digest)),
        findings,
        limitations,
        vec![],
    )
    .expect("the scanner constructs unique valid mechanism findings"))
}

fn is_plain_text_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

#[derive(Debug)]
enum InspectError {
    Open { path: PathBuf, source: io::Error },
    Read { path: PathBuf, source: io::Error },
    NotRegularFile(PathBuf),
    ArtifactTooLarge,
    NormalizationBufferAllocation,
    C2paBufferAllocation,
    NormalizationUtf8Invariant,
    Normalization(unicode_normalization::AnalysisError),
    C2pa(c2pa_provenance::AnalysisError),
}

impl fmt::Display for InspectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, source } => {
                write!(formatter, "could not open {:?}: {source}", path.display())
            }
            Self::Read { path, source } => {
                write!(formatter, "could not read {:?}: {source}", path.display())
            }
            Self::NotRegularFile(path) => {
                write!(
                    formatter,
                    "artifact is not a regular file: {:?}",
                    path.display()
                )
            }
            Self::ArtifactTooLarge => formatter.write_str("artifact byte length exceeds u64"),
            Self::NormalizationBufferAllocation => {
                formatter.write_str("could not allocate the whole-artifact normalization buffer")
            }
            Self::C2paBufferAllocation => {
                formatter.write_str("could not allocate the exact-byte C2PA artifact buffer")
            }
            Self::NormalizationUtf8Invariant => formatter
                .write_str("internal UTF-8 validation inconsistency while preparing normalization"),
            Self::Normalization(error) => write!(formatter, "normalization failed: {error}"),
            Self::C2pa(error) => write!(formatter, "C2PA inspection failed: {error}"),
        }
    }
}

impl Error for InspectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Read { source, .. } => Some(source),
            Self::NotRegularFile(_)
            | Self::ArtifactTooLarge
            | Self::NormalizationBufferAllocation
            | Self::C2paBufferAllocation
            | Self::NormalizationUtf8Invariant => None,
            Self::Normalization(error) => Some(error),
            Self::C2pa(error) => Some(error),
        }
    }
}

fn write_human(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    writeln!(writer, "scrub inspect")?;
    writeln!(writer)?;
    writeln!(writer, "Artifact")?;
    writeln!(
        writer,
        "  name    {}",
        terminal_safe(report.artifact().path())
    )?;
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
                    && finding.mechanism().id() != c2pa_provenance::MANIFEST_STORE_ID)
        })
        .collect();
    if notable_c2pa.is_empty() {
        if let Some(wrapper) = c2pa
            .iter()
            .copied()
            .find(|finding| finding.mechanism().id() == c2pa_provenance::TEXT_WRAPPER_ID)
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
        .find(|finding| finding.mechanism().id() == provider_watermark::MECHANISM_ID)
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
                terminal_safe(evidence.name()),
                terminal_safe(evidence.value())
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
                    terminal_safe(evidence.name()),
                    terminal_safe(evidence.value())
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
            writeln!(writer, "  limitation    {}", terminal_safe(limitation))?;
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
        unicode_bidi_control::MECHANISM_ID => "Bidi_Control",
        unicode_default_ignorable::MECHANISM_ID => "Default_Ignorable_Code_Point",
        unicode_normalization::NFC_MECHANISM_ID => "NFC difference",
        unicode_normalization::NFKC_MECHANISM_ID => "NFKC difference",
        c2pa_provenance::TEXT_WRAPPER_ID => "text manifest wrapper",
        c2pa_provenance::MANIFEST_STORE_ID => "manifest store",
        c2pa_provenance::MANIFEST_VALIDATION_ID => "manifest validation",
        c2pa_provenance::HARD_BINDING_ID => "hard binding",
        c2pa_provenance::CREDENTIAL_TRUST_ID => "credential trust",
        provider_watermark::MECHANISM_ID => "embedded text watermark",
        _ => mechanism_id,
    }
}

fn terminal_safe(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for scalar in value.chars() {
        if scalar.is_control() || ('\u{7f}'..='\u{9f}').contains(&scalar) {
            use std::fmt::Write as _;
            write!(output, "\\u{{{:x}}}", u32::from(scalar))
                .expect("writing to a String cannot fail");
        } else {
            output.push(scalar);
        }
    }
    output
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
    use scrub_report::FindingStatus;

    struct PartitionedReader<'a> {
        input: &'a [u8],
        chunks: Vec<usize>,
        offset: usize,
        read_index: usize,
    }

    impl<'a> PartitionedReader<'a> {
        fn new(input: &'a [u8], chunks: Vec<usize>) -> Self {
            Self {
                input,
                chunks,
                offset: 0,
                read_index: 0,
            }
        }
    }

    impl Read for PartitionedReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.offset == self.input.len() {
                return Ok(0);
            }
            let requested = self.chunks[self.read_index % self.chunks.len()];
            self.read_index += 1;
            let count = requested
                .min(output.len())
                .min(self.input.len() - self.offset);
            output[..count].copy_from_slice(&self.input[self.offset..self.offset + count]);
            self.offset += count;
            Ok(count)
        }
    }

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
    fn complete_inspection_is_invariant_under_arbitrary_read_partitions() {
        let input = "\u{754c}\u{1f600}e\u{301}\u{fb03}".as_bytes();
        let path = Path::new("partitioned-test-artifact");
        let expected = inspect_reader(path, &mut io::Cursor::new(input))
            .expect("unpartitioned inspection succeeds");

        for chunks in [vec![1], vec![2], vec![1, 3, 2, 5, 1, 4]] {
            let actual = inspect_reader(path, &mut PartitionedReader::new(input, chunks))
                .expect("partitioned inspection succeeds");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn a8_prefix_and_supplementary_selectors_are_partition_invariant() {
        let mut input = String::from("prefix");
        input.push('\u{feff}');
        for byte in b"C2PATXT\0"
            .iter()
            .copied()
            .chain([1])
            .chain(1_u32.to_be_bytes())
            .chain([255])
        {
            let scalar = if byte < 16 {
                0xfe00 + u32::from(byte)
            } else {
                0xe0100 + u32::from(byte) - 16
            };
            input.push(char::from_u32(scalar).expect("test mapping yields a scalar"));
        }

        let path = Path::new("partitioned-wrapper.txt");
        let expected = inspect_reader(path, &mut io::Cursor::new(input.as_bytes()))
            .expect("unpartitioned inspection succeeds");
        let wrapper = expected
            .findings()
            .iter()
            .find(|finding| finding.mechanism().id() == "c2pa.text_manifest_wrapper")
            .expect("wrapper finding exists");
        assert_eq!(wrapper.status(), FindingStatus::Present);

        for chunks in [vec![1], vec![2, 1, 3], vec![5, 1, 4, 2, 1]] {
            let actual =
                inspect_reader(path, &mut PartitionedReader::new(input.as_bytes(), chunks))
                    .expect("partitioned inspection succeeds");
            assert_eq!(actual, expected);
        }
    }
}
