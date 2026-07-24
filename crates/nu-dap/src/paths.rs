//! Path normalization shared by everything that compares file paths
//! (breakpoints, the source map, the parser's view of the script).
//!
//! `std::fs::canonicalize` on Windows returns verbatim paths (`\\?\C:\…`).
//! Those don't survive naive joining (forward slashes aren't separators in
//! verbatim form), which breaks nu's `source` resolution, and they'd have to
//! be stripped identically on every comparison anyway. So: canonicalize,
//! then strip the verbatim prefix once, here.

use std::path::Path;

/// Canonicalize and de-verbatim. Falls back to the input when the file
/// doesn't exist (or the name isn't a real path).
pub(crate) fn canonical(p: &Path) -> String {
    match std::fs::canonicalize(p) {
        Ok(c) => strip_verbatim(&c.to_string_lossy()),
        Err(_) => p.to_string_lossy().to_string(),
    }
}

pub(crate) fn canonical_str(p: &str) -> String {
    canonical(Path::new(p))
}

fn strip_verbatim(s: &str) -> String {
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::strip_verbatim;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(r"\\?\C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
    #[case(r"\\?\UNC\server\share\demo.nu", r"\\server\share\demo.nu")]
    #[case(r"C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
    #[case("/home/user/demo.nu", "/home/user/demo.nu")]
    fn strips_windows_verbatim_prefixes(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(strip_verbatim(input), expected);
    }
}
