use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use scrub_report::{ArtifactIdentity, Report, Sha256Digest, ToolIdentity};
use sha2::{Digest, Sha256};

mod unicode_bidi_control;
mod unicode_default_ignorable;
mod unicode_normalization;
mod utf8_stream;

const USAGE: &str = "Usage: scrub inspect <path> [--json]";
const UNEVALUATED_MECHANISMS_LIMITATION: &str = "Inspection currently evaluates Unicode 17.0.0 Default_Ignorable_Code_Point, Bidi_Control, NFC-difference, and NFKC-difference observations; confusable, sanitization, metadata, C2PA, statistical watermark, Claude-specific embedded watermark detection, and WaterLARP mechanisms are not evaluated.";

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
    let mut options = true;
    for argument in arguments {
        if options && argument == "--" {
            options = false;
        } else if options && argument == "--json" {
            if output == Output::Json {
                return Err(CliError::DuplicateJsonOption);
            }
            output = Output::Json;
        } else if options && argument.to_string_lossy().starts_with('-') {
            return Err(CliError::UnknownOption(argument));
        } else if path.replace(PathBuf::from(argument)).is_some() {
            return Err(CliError::TooManyPaths);
        }
    }

    let path = path.ok_or(CliError::MissingPath)?;
    Ok(InspectCommand { path, output })
}

#[derive(Debug, Eq, PartialEq)]
enum CliError {
    MissingCommand,
    UnknownCommand(OsString),
    MissingPath,
    TooManyPaths,
    UnknownOption(OsString),
    DuplicateJsonOption,
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

    let digest: [u8; 32] = hasher.finalize().into();
    let mut limitations = vec![UNEVALUATED_MECHANISMS_LIMITATION.to_owned()];
    let display_path = match path.to_str() {
        Some(path) => path.to_owned(),
        None => {
            limitations
                .push("The artifact path was lossily converted to Unicode for display.".to_owned());
            path.to_string_lossy().into_owned()
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
    Ok(Report::new(
        ToolIdentity::new("scrub", env!("CARGO_PKG_VERSION")),
        ArtifactIdentity::new(display_path, byte_length, Sha256Digest::from_bytes(digest)),
        vec![
            dicp_inspection.finish(valid_utf8),
            bidi_inspection.finish(valid_utf8),
            nfc_finding,
            nfkc_finding,
        ],
        limitations,
        vec![],
    ))
}

#[derive(Debug)]
enum InspectError {
    Open { path: PathBuf, source: io::Error },
    Read { path: PathBuf, source: io::Error },
    NotRegularFile(PathBuf),
    ArtifactTooLarge,
    NormalizationBufferAllocation,
    NormalizationUtf8Invariant,
    Normalization(unicode_normalization::AnalysisError),
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
            Self::NormalizationUtf8Invariant => formatter
                .write_str("internal UTF-8 validation inconsistency while preparing normalization"),
            Self::Normalization(error) => write!(formatter, "normalization failed: {error}"),
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
            | Self::NormalizationUtf8Invariant => None,
            Self::Normalization(error) => Some(error),
        }
    }
}

fn write_human(writer: &mut impl Write, report: &Report) -> Result<(), OutputError> {
    writeln!(writer, "artifact: {}", report.artifact().path())?;
    writeln!(writer, "bytes: {}", report.artifact().byte_length())?;
    writeln!(writer, "sha256: {}", report.artifact().content_sha256())?;
    for finding in report.findings() {
        writeln!(
            writer,
            "mechanism: {} (Unicode {})",
            mechanism_display_name(finding.mechanism().id()),
            finding.mechanism().version()
        )?;
        writeln!(writer, "status: {}", finding.status())?;
        for evidence in finding.evidence() {
            writeln!(writer, "evidence: {}={}", evidence.name(), evidence.value())?;
        }
        for limitation in finding.limitations() {
            writeln!(writer, "finding limitation: {limitation}")?;
        }
    }
    for limitation in report.limitations() {
        writeln!(writer, "limitation: {limitation}")?;
    }
    Ok(())
}

fn mechanism_display_name(mechanism_id: &str) -> &str {
    match mechanism_id {
        unicode_bidi_control::MECHANISM_ID => "Bidi_Control",
        unicode_default_ignorable::MECHANISM_ID => "Default_Ignorable_Code_Point",
        unicode_normalization::NFC_MECHANISM_ID => "NFC difference",
        unicode_normalization::NFKC_MECHANISM_ID => "NFKC difference",
        _ => mechanism_id,
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
}
