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

/// Sides of the parent/child NUON handshake, from [`STRUCTURED_IO_ENV`] or `--structured-io=`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuredIoMode {
    pub input: bool,
    pub output: bool,
}

impl StructuredIoMode {
    pub fn both() -> Self {
        Self {
            input: true,
            output: true,
        }
    }

    pub fn any(self) -> bool {
        self.input || self.output
    }

    /// Parse a handshake value: `1`/`true`/`inout`, `in`, `out`, or anything else as off.
    pub fn from_flag_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "false" | "off" | "0" => Self::default(),
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
