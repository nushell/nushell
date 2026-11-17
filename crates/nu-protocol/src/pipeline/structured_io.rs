#[cfg(feature = "os")]
use crate::ByteStreamSource;
use crate::{PipelineData, ShellError, Span};

/// Environment variable a parent `nu` sets on a child `nu` it spawns, holding the
/// [`StructuredIoMode`] it wants (`in`, `out` or `1`).
///
/// The parent sets it only on spawns whose stdin it fills with a frame or whose stdout it reads,
/// and removes it from every other spawn. The child reads it at startup and keeps it out of its
/// own `$env`, so the child's own children never see it.
pub const STRUCTURED_IO_ENV: &str = "NU_STRUCTURED_IO";

/// Metadata `content_type` of a Nushell child's output that may start with a NUON frame. The
/// parent tags the output this way when the next command is an external, instead of waiting to
/// decode it, and the command that reads it decodes it.
pub const STRUCTURED_IO_CONTENT_TYPE: &str = "application/x-nushell-structured-io";

/// Values `--structured-io` accepts. [`StructuredIoMode::from_flag_str`] reads them.
pub const STRUCTURED_IO_FLAG_VALUES: &[&str] =
    &["1", "true", "inout", "in", "out", "false", "off", "0"];

/// Turns a Nushell child's tagged output (see [`is_structured_io_child`]) into the value it sent,
/// or into an untagged byte stream when it sent text. Decoding parses NUON, which this crate
/// can't do, so the crate that tags the output provides it (see
/// [`EngineState::set_structured_io_decoder`](crate::engine::EngineState::set_structured_io_decoder)).
pub type StructuredIoDecoder = fn(PipelineData, Span) -> Result<PipelineData, ShellError>;

/// Whether `data` is a Nushell child's output that `run-external` tagged with
/// [`STRUCTURED_IO_CONTENT_TYPE`] for the command that reads it to decode.
#[cfg(feature = "os")]
pub fn is_structured_io_child(data: &PipelineData) -> bool {
    matches!(data, PipelineData::ByteStream(stream, Some(metadata))
        if metadata.content_type.as_deref() == Some(STRUCTURED_IO_CONTENT_TYPE)
            && matches!(stream.source(), ByteStreamSource::Child(..)))
}

/// Sides of the parent/child NUON handshake, from [`STRUCTURED_IO_ENV`] or `--structured-io=`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuredIoMode {
    /// The parent writes the child's input to its stdin as one frame.
    pub input: bool,
    /// The child writes its final value to stdout as one frame, when nothing else went there.
    pub output: bool,
}

impl StructuredIoMode {
    /// Both sides on: `--structured-io=1`.
    pub fn both() -> Self {
        Self {
            input: true,
            output: true,
        }
    }

    /// Whether either side is on.
    pub fn any(self) -> bool {
        self.input || self.output
    }

    /// Parse a handshake value: `1`/`true`/`inout`, `in`, `out`, or anything else as off.
    ///
    /// The CLI rejects other values for `--structured-io` (see [`STRUCTURED_IO_FLAG_VALUES`]), but
    /// [`STRUCTURED_IO_ENV`] and a child's command line seen by its parent can still hold one.
    pub fn from_flag_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "inout" => Self::both(),
            "in" => Self {
                input: true,
                output: false,
            },
            "out" => Self {
                input: false,
                output: true,
            },
            _ => Self::default(),
        }
    }

    /// Value for `--structured-io=...`, or `None` when the protocol is off.
    pub fn as_flag_str(self) -> Option<&'static str> {
        match (self.input, self.output) {
            (true, true) => Some("1"),
            (true, false) => Some("in"),
            (false, true) => Some("out"),
            (false, false) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_handshake_values() {
        assert_eq!(
            StructuredIoMode::from_flag_str("1"),
            StructuredIoMode::both()
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("in"),
            StructuredIoMode {
                input: true,
                output: false
            }
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("out"),
            StructuredIoMode {
                input: false,
                output: true
            }
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("false"),
            StructuredIoMode::default()
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("off"),
            StructuredIoMode::default()
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("0"),
            StructuredIoMode::default()
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("IN"),
            StructuredIoMode {
                input: true,
                output: false
            }
        );
        assert_eq!(
            StructuredIoMode::from_flag_str("nope"),
            StructuredIoMode::default()
        );
    }

    #[test]
    fn roundtrips_flag_str() {
        for mode in [
            StructuredIoMode::default(),
            StructuredIoMode {
                input: true,
                output: false,
            },
            StructuredIoMode {
                input: false,
                output: true,
            },
            StructuredIoMode::both(),
        ] {
            match mode.as_flag_str() {
                None => assert!(!mode.any()),
                Some(value) => assert_eq!(StructuredIoMode::from_flag_str(value), mode),
            }
        }
    }
}
