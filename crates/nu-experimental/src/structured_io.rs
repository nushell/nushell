/// Which sides of the child pipeline should use NUON instead of raw bytes / tables.
///
/// The parent `run-external` passes this as `--structured-io=in|out|1` on the child
/// `nu` command line. A CLI flag is used instead of an environment variable so the
/// handshake does not call `set_var` / `remove_var` on the current process.
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
        match value.trim() {
            "false" | "off" | "0" => Self::default(),
            "1" | "true" | "TRUE" | "inout" => Self::both(),
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
