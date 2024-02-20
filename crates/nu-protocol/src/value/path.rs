use serde::Deserialize;
use std::fmt::Display;

#[derive(Debug, Clone, Deserialize)]
pub enum NuGlob {
    /// A quoted path(except backtick), in this case, nushell shouldn't auto-expand path.
    Quoted(String),
    /// An unquoted path, in this case, nushell should auto-expand path.
    UnQuoted(String),
}

impl NuGlob {
    pub fn strip_ansi_string_unlikely(self) -> Self {
        match self {
            NuGlob::Quoted(s) => NuGlob::Quoted(nu_utils::strip_ansi_string_unlikely(s)),
            NuGlob::UnQuoted(s) => NuGlob::UnQuoted(nu_utils::strip_ansi_string_unlikely(s)),
        }
    }
}

impl AsRef<str> for NuGlob {
    fn as_ref(&self) -> &str {
        match self {
            NuGlob::Quoted(s) | NuGlob::UnQuoted(s) => s,
        }
    }
}

impl Display for NuGlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
