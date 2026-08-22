/// Environment variable used as the parent/child structured IO handshake.
///
/// Set by the parent `run-external` when the experimental `structured-io` option is on
/// and the child is Nushell. Consumed (and unset) by the child at startup so it is not
/// inherited by grandchildren.
pub const STRUCTURED_IO_ENV: &str = "NU_STRUCTURED_IO";

/// Which sides of the child pipeline should use NUON instead of raw bytes / tables.
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
    pub fn from_env_str(value: &str) -> Self {
        match value.trim() {
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

    pub fn from_os_env() -> Self {
        std::env::var(STRUCTURED_IO_ENV)
            .ok()
            .map(|value| Self::from_env_str(&value))
            .unwrap_or_default()
    }

    /// Value to store in [`STRUCTURED_IO_ENV`], or `None` when the protocol is off.
    pub fn as_env_str(self) -> Option<&'static str> {
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
            StructuredIoMode::from_env_str("1"),
            StructuredIoMode::both()
        );
        assert_eq!(
            StructuredIoMode::from_env_str("in"),
            StructuredIoMode {
                input: true,
                output: false
            }
        );
        assert_eq!(
            StructuredIoMode::from_env_str("out"),
            StructuredIoMode {
                input: false,
                output: true
            }
        );
        assert_eq!(
            StructuredIoMode::from_env_str("nope"),
            StructuredIoMode::default()
        );
    }

    #[test]
    fn roundtrips_env_str() {
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
            match mode.as_env_str() {
                None => assert!(!mode.any()),
                Some(value) => assert_eq!(StructuredIoMode::from_env_str(value), mode),
            }
        }
    }
}
