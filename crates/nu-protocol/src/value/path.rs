use serde::Deserialize;
use std::fmt::Display;

#[derive(Debug, Clone, Deserialize)]
pub enum NuGlob {
    /// A quoted path(except backtick), in this case, nushell shouldn't auto-expand path.
    NoExpand(String),
    /// An unquoted path, in this case, nushell should auto-expand path.
    NeedExpand(String),
}

impl NuGlob {
    pub fn strip_ansi_string_unlikely(self) -> Self {
        match self {
            NuGlob::NoExpand(s) => NuGlob::NoExpand(nu_utils::strip_ansi_string_unlikely(s)),
            NuGlob::NeedExpand(s) => NuGlob::NeedExpand(nu_utils::strip_ansi_string_unlikely(s)),
        }
    }
}

impl AsRef<str> for NuGlob {
    fn as_ref(&self) -> &str {
        match self {
            NuGlob::NoExpand(s) | NuGlob::NeedExpand(s) => s,
        }
    }
}

impl Display for NuGlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
