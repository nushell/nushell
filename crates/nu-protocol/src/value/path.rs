#[derive(Debug, Clone)]
pub enum NuPath {
    Quoted(String),
    UnQuoted(String),
}

impl AsRef<str> for NuPath {
    fn as_ref(&self) -> &str {
        match self {
            NuPath::Quoted(s) | NuPath::UnQuoted(s) => s,
        }
    }
}
