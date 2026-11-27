//! What a parent `nu` and a child `nu` started with `--structured-io` agree on to pass values
//! between them. The channel itself lives in `nu-command`, next to `run-external`.

/// Environment variable a parent `nu` sets on a child `nu` it started with `--structured-io`. It
/// holds a [`StructuredIoLink`]. The child keeps it out of its own `$env`, so the child's own
/// children never see it.
pub const STRUCTURED_IO_ENV: &str = "NU_STRUCTURED_IO";

/// The argument that asks for structured IO: on the command line of the `nu` binary, or in the
/// shebang line of a script that `nu` runs. Only the parent reads it; the child follows
/// [`STRUCTURED_IO_ENV`].
pub const STRUCTURED_IO_FLAG: &str = "--structured-io";

/// Version of the channel's messages. A child that doesn't know the version a parent asks for
/// fails instead of guessing.
pub const STRUCTURED_IO_VERSION: u32 = 1;

/// Which way values go between a parent `nu` and a child `nu`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuredIoMode {
    /// The parent sends the child its input.
    pub input: bool,
    /// The child sends the parent its result.
    pub output: bool,
}

impl StructuredIoMode {
    /// Whether either way is on.
    pub fn any(self) -> bool {
        self.input || self.output
    }
}

/// How a child reaches the channel its parent opened: the [`STRUCTURED_IO_ENV`] value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredIoLink {
    /// Sent first on every connection, so the parent only talks to the process it started.
    pub token: String,
    /// The ways the parent opened.
    pub mode: StructuredIoMode,
    /// Name of the parent's local socket.
    pub socket: String,
}

impl StructuredIoLink {
    /// The [`STRUCTURED_IO_ENV`] value: version, token, ways (`i`, `o` or `io`) and socket name,
    /// split by `;`. The socket name comes last since a path may hold a `;`.
    pub fn to_env_value(&self) -> String {
        let ways = match (self.mode.input, self.mode.output) {
            (true, true) => "io",
            (true, false) => "i",
            (false, true) => "o",
            (false, false) => "",
        };
        format!(
            "{STRUCTURED_IO_VERSION};{};{ways};{}",
            self.token, self.socket
        )
    }

    /// Reads a [`STRUCTURED_IO_ENV`] value written by [`to_env_value`](Self::to_env_value).
    pub fn from_env_value(value: &str) -> Result<Self, String> {
        let mut fields = value.splitn(4, ';');
        let (Some(version), Some(token), Some(ways), Some(socket)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(format!("`{value}` is not a structured IO link"));
        };
        if version != STRUCTURED_IO_VERSION.to_string() {
            return Err(format!(
                "the calling nu uses structured IO version {version}, but this nu only knows version {STRUCTURED_IO_VERSION}"
            ));
        }
        let mode = StructuredIoMode {
            input: ways.contains('i'),
            output: ways.contains('o'),
        };
        if token.is_empty() || socket.is_empty() || !mode.any() {
            return Err(format!("`{value}` is not a structured IO link"));
        }
        Ok(Self {
            token: token.into(),
            mode,
            socket: socket.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_roundtrips_through_env_value() {
        for (input, output) in [(true, false), (false, true), (true, true)] {
            let link = StructuredIoLink {
                token: "0123abcd".into(),
                mode: StructuredIoMode { input, output },
                socket: "/tmp/nu.1.2/a;b.sock".into(),
            };
            assert_eq!(
                StructuredIoLink::from_env_value(&link.to_env_value()),
                Ok(link)
            );
        }
    }

    #[test]
    fn rejects_other_versions_and_garbage() {
        assert!(StructuredIoLink::from_env_value("2;t;io;/tmp/s").is_err());
        assert!(StructuredIoLink::from_env_value("1;t;;/tmp/s").is_err());
        assert!(StructuredIoLink::from_env_value("1;;io;/tmp/s").is_err());
        assert!(StructuredIoLink::from_env_value("out").is_err());
    }
}
