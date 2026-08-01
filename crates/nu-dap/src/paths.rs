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
