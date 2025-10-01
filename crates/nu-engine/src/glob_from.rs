use nu_glob::MatchOptions;
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::{NuGlob, ShellError, Signals, Span, Spanned, shell_error::io::IoError};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

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
    signals: Signals,
) -> Result<
    (
        Option<PathBuf>,
        Box<dyn Iterator<Item = Result<PathBuf, ShellError>> + Send>,
    ),
    ShellError,
> {
    let no_glob_for_pattern = matches!(pattern.item, NuGlob::DoNotExpand(_));
    let pattern_span = pattern.span;
    let (prefix, pattern) = if nu_glob::is_glob(pattern.item.as_ref()) {
        // Pattern contains glob, split it
        let mut p = PathBuf::new();
        let path = PathBuf::from(&pattern.item.as_ref());
        let components = path.components();
        let mut counter = 0;

        for c in components {
            if let Component::Normal(os) = c
                && nu_glob::is_glob(os.to_string_lossy().as_ref())
            {
                break;
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
        let path = expand_path_with(p, cwd, pattern.item.is_expand());
        let escaped_prefix = PathBuf::from(nu_glob::Pattern::escape(&path.to_string_lossy()));

        (Some(path), escaped_prefix.join(just_pattern))
    } else {
        let path = PathBuf::from(&pattern.item.as_ref());
        let path = expand_path_with(path, cwd, pattern.item.is_expand());
        let is_symlink = match fs::symlink_metadata(&path) {
            Ok(attr) => attr.file_type().is_symlink(),
            Err(_) => false,
        };

        if is_symlink {
            (path.parent().map(|parent| parent.to_path_buf()), path)
        } else {
            let path = match canonicalize_with(path.clone(), cwd) {
                Ok(p) if nu_glob::is_glob(p.to_string_lossy().as_ref()) => {
                    // our path might contain glob metacharacters too.
                    // in such case, we need to escape our path to make
                    // glob work successfully
                    PathBuf::from(nu_glob::Pattern::escape(&p.to_string_lossy()))
                }
                Ok(p) => p,
                Err(err) => {
                    return Err(IoError::new(err, pattern_span, path).into());
                }
            };
            (path.parent().map(|parent| parent.to_path_buf()), path)
        }
    };

    let pattern = pattern.to_string_lossy().to_string();
    let glob_options = options.unwrap_or_default();

    let glob = nu_glob::glob_with(&pattern, glob_options, signals).map_err(|e| {
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
