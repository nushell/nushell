use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use nu_glob::MatchOptions;
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::{ShellError, Span, Spanned};

const GLOB_CHARS: &[char] = &['*', '?', '['];

/// This function is like `nu_glob::glob` from the `glob` crate, except it is relative to a given cwd.
///
/// It returns a tuple of two values: the first is an optional prefix that the expanded filenames share.
/// This prefix can be removed from the front of each value to give an approximation of the relative path
/// to the user
///
/// The second of the two values is an iterator over the matching filepaths.
#[allow(clippy::type_complexity)]
pub fn glob_from(
    pattern: &Spanned<String>,
    cwd: &Path,
    span: Span,
    options: Option<MatchOptions>,
) -> Result<
    (
        Option<PathBuf>,
        Box<dyn Iterator<Item = Result<PathBuf, ShellError>> + Send>,
    ),
    ShellError,
> {
    let path = PathBuf::from(&pattern.item);
    let path = expand_path_with(path, cwd);
    let is_symlink = match fs::symlink_metadata(&path) {
        Ok(attr) => attr.file_type().is_symlink(),
        Err(_) => false,
    };

    let expanded_cwd = expand_path_with(".", cwd);

    // Check for brackets first
    let (prefix, pattern) = if path.to_string_lossy().contains(GLOB_CHARS) {
        // Path is a glob pattern => do not check for existence
        // Select the longest prefix until the first '*'
        let mut p = PathBuf::new();
        let components = path.components();
        let mut counter = 0;

        let mut cwd_components = expanded_cwd.components();

        // Get the path up to the pattern which we'll call the prefix
        for c in components {
            let cwd = cwd_components.next();
            if let Component::Normal(os) = c {
                // Only the following pattern is glob pattern:
                // (1) Not a part of expanded_cwd, e.g. ls under `/[test]/`
                // (2) Contain glob_chars
                let part_of_cwd = match cwd {
                    Some(Component::Normal(cwd)) => os == cwd,
                    _ => false,
                };
                if !part_of_cwd && os.to_string_lossy().contains(GLOB_CHARS) {
                    break;
                }
            }
            p.push(c);
            counter += 1;
        }

        // Let's separate the pattern from the path and we'll call this the pattern
        let mut just_pattern = PathBuf::new();
        for c in counter..path.components().count() {
            if let Some(comp) = path.components().nth(c) {
                just_pattern.push(comp);
            }
        }

        (Some(p), just_pattern)
    } else if is_symlink {
        (path.parent().map(|parent| parent.to_path_buf()), path)
    } else {
        let path = if let Ok(p) = canonicalize_with(path, cwd) {
            p
        } else {
            return Err(ShellError::DirectoryNotFound(pattern.span, None));
        };
        (path.parent().map(|parent| parent.to_path_buf()), path)
    };

    let pattern = pattern.to_string_lossy().to_string();
    let glob_options = options.unwrap_or_else(MatchOptions::new);

    let glob = nu_glob::glob_with(&pattern, glob_options).map_err(|err| {
        nu_protocol::ShellError::GenericError(
            "Error extracting glob pattern".into(),
            err.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })?;

    Ok((
        prefix,
        Box::new(glob.map(move |x| match x {
            Ok(v) => Ok(v),
            Err(err) => Err(nu_protocol::ShellError::GenericError(
                "Error extracting glob pattern".into(),
                err.to_string(),
                Some(span),
                None,
                Vec::new(),
            )),
        })),
    ))
}
