use serde::Deserialize;
use std::fmt::Display;

/// A simple wrapper to String.
///
/// But it tracks if the string is originally quoted.
/// So commands can make decision on path auto-expanding behavior.
#[derive(Debug, Clone, Deserialize)]
pub enum NuPath {
    /// A quoted path(except backtick), in this case, nushell shouldn't auto-expand path.
    Quoted(String),
    /// An unquoted path, in this case, nushell should auto-expand path.
    UnQuoted(String),
}

impl NuPath {
    pub fn strip_ansi_string_unlikely(self) -> Self {
        match self {
            NuPath::Quoted(s) => NuPath::Quoted(nu_utils::strip_ansi_string_unlikely(s)),
            NuPath::UnQuoted(s) => NuPath::UnQuoted(nu_utils::strip_ansi_string_unlikely(s)),
        }
    }
}

impl AsRef<str> for NuPath {
    fn as_ref(&self) -> &str {
        match self {
            NuPath::Quoted(s) | NuPath::UnQuoted(s) => s,
        }
    }
}

impl Display for NuPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
