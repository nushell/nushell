//! One comparison key per file, for breakpoints and the source map. The same
//! file arrives spelled differently: absolute from the client, relative or
//! tilde'd from the engine.

use std::path::Path;

/// Canonicalize into a comparison key; unresolvable names pass through.
///
/// [`nu_path::canonicalize_with`] rather than std, so the Windows `\\?\`
/// verbatim prefix is stripped — verbatim paths break nu's `source` joining.
/// The base is the process cwd rather than an argument, because
/// `setBreakpoints` can arrive before `launch`. Unresolvable names pass
/// through instead of expanding lexically: `<entry-call>` is not a file.
pub(crate) fn canonical(p: impl AsRef<Path>) -> String {
    let p = p.as_ref();
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    nu_path::canonicalize_with(p, cwd)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::paths`].

    use super::canonical;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// The point of the module: two spellings of the same file produce one key, so
    /// a breakpoint set on an absolute path matches a `source`d relative name.
    #[rstest]
    #[case::bare("Cargo.toml")]
    #[case::dot_prefixed("./Cargo.toml")]
    #[case::through_a_parent("src/../Cargo.toml")]
    fn spellings_of_one_file_agree(#[case] spelling: &str) {
        let expected = canonical("Cargo.toml");
        assert_eq!(canonical(spelling), expected);
    }

    /// Canonicalizing must not leave the Windows verbatim prefix behind: `\\?\`
    /// paths don't survive naive joining, which breaks nu's `source` resolution.
    /// `std::fs::canonicalize` returns exactly that form, so this is the reason
    /// the module delegates to `nu_path` rather than calling std directly.
    #[test]
    fn the_windows_verbatim_prefix_is_gone() {
        let key = canonical("Cargo.toml");
        assert!(!key.starts_with(r"\\?\"), "verbatim prefix survived: {key}");
        assert!(
            std::path::Path::new(&key).is_absolute(),
            "not absolute: {key}"
        );
    }

    /// Not every name in `engine_state.files()` is a real path — `<entry-call>` is
    /// synthetic — so an unresolvable name comes back untouched rather than being
    /// joined onto the cwd, which would invent a file that never existed.
    #[rstest]
    #[case::synthetic("<entry-call>")]
    #[case::missing_file("does-not-exist-9d1f.nu")]
    fn unresolvable_names_pass_through(#[case] name: &str) {
        assert_eq!(canonical(name), name);
    }
}
