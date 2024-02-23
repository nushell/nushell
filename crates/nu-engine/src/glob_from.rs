use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use nu_glob::MatchOptions;
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::{NuGlob, ShellError, Span, Spanned};

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
    pattern: &Spanned<NuGlob>,
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
    let no_glob_for_pattern = matches!(pattern.item, NuGlob::DoNotExpand(_));
    let (prefix, pattern) = if pattern.item.as_ref().contains(GLOB_CHARS) {
        // Pattern contains glob, split it
        let mut p = PathBuf::new();
        let path = PathBuf::from(&pattern.item.as_ref());
        let components = path.components();
        let mut counter = 0;

        for c in components {
            if let Component::Normal(os) = c {
                if os.to_string_lossy().contains(GLOB_CHARS) {
                    break;
                }
            }
            p.push(c);
            counter += 1;
        }

        let mut just_pattern = PathBuf::new();
        for c in counter..path.components().count() {
            if let Some(comp) = path.components().nth(c) {
                just_pattern.push(comp);
            }
        }
        if no_glob_for_pattern {
            just_pattern = PathBuf::from(nu_glob::Pattern::escape(&just_pattern.to_string_lossy()));
        }

        // Now expand `p` to get full prefix
        let path = expand_path_with(p, cwd);
        let escaped_prefix = PathBuf::from(nu_glob::Pattern::escape(&path.to_string_lossy()));

        (Some(path), escaped_prefix.join(just_pattern))
    } else {
        let path = PathBuf::from(&pattern.item.as_ref());
        let path = expand_path_with(path, cwd);
        let is_symlink = match fs::symlink_metadata(&path) {
            Ok(attr) => attr.file_type().is_symlink(),
            Err(_) => false,
        };

        if is_symlink {
            (path.parent().map(|parent| parent.to_path_buf()), path)
        } else {
            let path = if let Ok(p) = canonicalize_with(path.clone(), cwd) {
                if p.to_string_lossy().contains(GLOB_CHARS) {
                    // our path might contains GLOB_CHARS too
                    // in such case, we need to escape our path to make
                    // glob work successfully
                    PathBuf::from(nu_glob::Pattern::escape(&p.to_string_lossy()))
                } else {
                    p
                }
            } else {
                return Err(ShellError::DirectoryNotFound {
                    dir: path.to_string_lossy().to_string(),
                    span: pattern.span,
                });
            };
            (path.parent().map(|parent| parent.to_path_buf()), path)
        }
    };

    let pattern = pattern.to_string_lossy().to_string();
    let glob_options = options.unwrap_or_default();

    let glob = nu_glob::glob_with(&pattern, glob_options).map_err(|e| {
        nu_protocol::ShellError::GenericError {
            error: "Error extracting glob pattern".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        }
    })?;

    Ok((
        prefix,
        Box::new(glob.map(move |x| match x {
            Ok(v) => Ok(v),
            Err(e) => Err(nu_protocol::ShellError::GenericError {
                error: "Error extracting glob pattern".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            }),
        })),
    ))
}
