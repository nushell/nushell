use serde::Deserialize;
use std::fmt::Display;

// Introduce this `NuGlob` enum rather than using `Value::Glob` directlry
// So we can handle glob easily without considering too much variant of `Value` enum.
#[derive(Debug, Clone, Deserialize)]
pub enum NuGlob {
    /// Don't expand the glob pattern, normally it includes a quoted string(except backtick)
    /// And a variable that doesn't annotated with `glob` type
    NoExpand(String),
    /// A glob pattern that is required to expand, it includes bare word
    /// And a variable which is annotated with `glob` type
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
