use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use scrub_report::{ArtifactIdentity, Report, Sha256Digest, ToolIdentity, human_safe};
use sha2::{Digest, Sha256};

mod c2pa_provenance;
mod provider_watermark;
mod unicode_bidi_control;
mod unicode_default_ignorable;
mod unicode_normalization;
mod utf8_stream;

pub const C2PA_TEXT_WRAPPER_ID: &str = c2pa_provenance::TEXT_WRAPPER_ID;
pub const C2PA_MANIFEST_STORE_ID: &str = c2pa_provenance::MANIFEST_STORE_ID;
pub const C2PA_MANIFEST_VALIDATION_ID: &str = c2pa_provenance::MANIFEST_VALIDATION_ID;
pub const C2PA_HARD_BINDING_ID: &str = c2pa_provenance::HARD_BINDING_ID;
pub const C2PA_CREDENTIAL_TRUST_ID: &str = c2pa_provenance::CREDENTIAL_TRUST_ID;
pub const PROVIDER_WATERMARK_ID: &str = provider_watermark::MECHANISM_ID;
pub const UNICODE_BIDI_CONTROL_ID: &str = unicode_bidi_control::MECHANISM_ID;
pub const UNICODE_DEFAULT_IGNORABLE_ID: &str = unicode_default_ignorable::MECHANISM_ID;
pub const UNICODE_NFC_DIFFERENCE_ID: &str = unicode_normalization::NFC_MECHANISM_ID;
pub const UNICODE_NFKC_DIFFERENCE_ID: &str = unicode_normalization::NFKC_MECHANISM_ID;

const UNEVALUATED_MECHANISMS_LIMITATION: &str = "Inspection evaluates the listed Unicode and C2PA mechanisms and represents Claude provider-detector availability; confusables, sanitization, unrelated metadata, public-reference statistical detectors, and WaterLARP mechanisms are not evaluated.";

pub fn inspect_file(path: &Path) -> Result<Report, InspectError> {
    let path_metadata = fs::metadata(path).map_err(|source| {
        InspectError::new(InspectErrorKind::Open {
            path: path.to_path_buf(),
            source,
        })
    })?;
    if !path_metadata.is_file() {
        return Err(InspectError::new(InspectErrorKind::NotRegularFile(
            path.to_path_buf(),
        )));
    }

    let mut file = File::open(path).map_err(|source| {
        InspectError::new(InspectErrorKind::Open {
            path: path.to_path_buf(),
            source,
        })
    })?;
    let metadata = file.metadata().map_err(|source| {
        InspectError::new(InspectErrorKind::Read {
            path: path.to_path_buf(),
            source,
        })
    })?;
    if !metadata.is_file() {
        return Err(InspectError::new(InspectErrorKind::NotRegularFile(
            path.to_path_buf(),
        )));
    }

    inspect_reader(path, &mut file)
}

/// Inspects exactly the bytes supplied by `reader` through the production
/// ingestion path. This boundary exists so deterministic tests and fuzzing can
/// vary legal `Read` partitions without introducing another scanner.
pub fn inspect_reader(path: &Path, reader: &mut impl Read) -> Result<Report, InspectError> {
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
        let count = reader.read(&mut buffer).map_err(|source| {
            InspectError::new(InspectErrorKind::Read {
                path: path.to_path_buf(),
                source,
            })
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
                    let previous = usize::try_from(byte_length)
                        .map_err(|_| InspectError::new(InspectErrorKind::ArtifactTooLarge))?;
                    let required = previous
                        .checked_add(count)
                        .ok_or_else(|| InspectError::new(InspectErrorKind::ArtifactTooLarge))?;
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
            .map_err(|_| InspectError::new(InspectErrorKind::ArtifactTooLarge))?;
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
        let count = u64::try_from(count)
            .map_err(|_| InspectError::new(InspectErrorKind::ArtifactTooLarge))?;
        byte_length = byte_length
            .checked_add(count)
            .ok_or_else(|| InspectError::new(InspectErrorKind::ArtifactTooLarge))?;
    }

    if normalization_buffer_failed {
        return Err(InspectError::new(
            InspectErrorKind::NormalizationBufferAllocation,
        ));
    }
    if c2pa_buffer_failed {
        return Err(InspectError::new(InspectErrorKind::C2paBufferAllocation));
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
            .map_err(|_| InspectError::new(InspectErrorKind::NormalizationUtf8Invariant))?;
        unicode_normalization::valid_findings(input)
            .map_err(|error| InspectError::new(InspectErrorKind::Normalization(error)))?
    } else {
        unicode_normalization::invalid_findings()
    };
    let textual_artifact = c2pa_binary.is_none() && (valid_utf8 || is_plain_text_path(path));
    let c2pa_findings = if let Some((format, bytes)) = c2pa_binary {
        c2pa_provenance::binary_format_findings(format, &bytes)
            .map_err(|error| InspectError::new(InspectErrorKind::C2pa(error)))?
    } else if valid_utf8 {
        let input = std::str::from_utf8(&normalization_bytes)
            .map_err(|_| InspectError::new(InspectErrorKind::NormalizationUtf8Invariant))?;
        c2pa_provenance::valid_text_findings(input)
            .map_err(|error| InspectError::new(InspectErrorKind::C2pa(error)))?
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
pub struct InspectError {
    kind: InspectErrorKind,
}

impl InspectError {
    fn new(kind: InspectErrorKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug)]
enum InspectErrorKind {
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
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
        match &self.kind {
            InspectErrorKind::Open { path, source } => {
                write!(
                    formatter,
                    "could not open {}: {}",
                    human_safe(&path.to_string_lossy()),
                    human_safe(&source.to_string())
                )
            }
            InspectErrorKind::Read { path, source } => {
                write!(
                    formatter,
                    "could not read {}: {}",
                    human_safe(&path.to_string_lossy()),
                    human_safe(&source.to_string())
                )
            }
            InspectErrorKind::NotRegularFile(path) => {
                write!(
                    formatter,
                    "artifact is not a regular file: {}",
                    human_safe(&path.to_string_lossy())
                )
            }
            InspectErrorKind::ArtifactTooLarge => {
                formatter.write_str("artifact byte length exceeds u64")
            }
            InspectErrorKind::NormalizationBufferAllocation => {
                formatter.write_str("could not allocate the whole-artifact normalization buffer")
            }
            InspectErrorKind::C2paBufferAllocation => {
                formatter.write_str("could not allocate the exact-byte C2PA artifact buffer")
            }
            InspectErrorKind::NormalizationUtf8Invariant => formatter
                .write_str("internal UTF-8 validation inconsistency while preparing normalization"),
            InspectErrorKind::Normalization(error) => {
                write!(
                    formatter,
                    "normalization failed: {}",
                    human_safe(&error.to_string())
                )
            }
            InspectErrorKind::C2pa(error) => write!(
                formatter,
                "C2PA inspection failed: {}",
                human_safe(&error.to_string())
            ),
        }
    }
}

impl Error for InspectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            InspectErrorKind::Open { source, .. } | InspectErrorKind::Read { source, .. } => {
                Some(source)
            }
            InspectErrorKind::NotRegularFile(_)
            | InspectErrorKind::ArtifactTooLarge
            | InspectErrorKind::NormalizationBufferAllocation
            | InspectErrorKind::C2paBufferAllocation
            | InspectErrorKind::NormalizationUtf8Invariant => None,
            InspectErrorKind::Normalization(error) => Some(error),
            InspectErrorKind::C2pa(error) => Some(error),
        }
    }
}
