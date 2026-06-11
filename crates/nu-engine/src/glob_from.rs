use nu_glob::MatchOptions;
use nu_path::{absolute_with, expand_path_with};
use nu_protocol::{
    NuGlob, ShellError, Signals, Span, Spanned, shell_error::generic::GenericError,
    shell_error::io::IoError,
};
use std::{
    fs, io,
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
    let (prefix, pattern) = if nu_glob::is_glob_with_backend(pattern.item.as_ref()) {
        // Pattern contains glob, split it
        let mut p = PathBuf::new();
        let path = PathBuf::from(&pattern.item.as_ref());
        let components = path.components();
        let mut counter = 0;

        for c in components {
            if let Component::Normal(os) = c
                && nu_glob::is_glob_with_backend(os.to_string_lossy().as_ref())
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
            just_pattern = PathBuf::from(nu_glob::escape_with_backend(
                &just_pattern.to_string_lossy(),
            ));
        }

        // Now expand `p` to get full prefix
        let path = expand_path_with(p, cwd, pattern.item.is_expand());
        let escaped_prefix = PathBuf::from(nu_glob::escape_with_backend(&path.to_string_lossy()));

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
            let path = match absolute_with(path.clone(), cwd) {
                Ok(p) if p.exists() => {
                    if nu_glob::is_glob_with_backend(p.to_string_lossy().as_ref()) {
                        // our path might contain glob metacharacters too.
                        // in such case, we need to escape our path to make
                        // glob work successfully
                        PathBuf::from(nu_glob::escape_with_backend(&p.to_string_lossy()))
                    } else {
                        p
                    }
                }
                Ok(_) => {
                    return Err(IoError::new(
                        io::Error::from(io::ErrorKind::NotFound),
                        pattern_span,
                        path,
                    )
                    .into());
                }
                Err(err) => {
                    return Err(IoError::new(err, pattern_span, path).into());
                }
            };
            (path.parent().map(|parent| parent.to_path_buf()), path)
        }
    };

    let pattern = pattern.to_string_lossy().to_string();

    if nu_experimental::DC_GLOB.get() {
        let pattern_path = PathBuf::from(&pattern);
        // If the resolved pattern is an existing path, return it directly.
        // Passing a plain path to glob_from_interruptible makes the traversal engine
        // call read_dir() on it, which either fails with "Not a directory" (for files)
        // or iterates the directory's contents instead of matching the directory itself
        // (for directories), both of which produce incorrect empty results.
        if pattern_path.exists() {
            return Ok((prefix, Box::new(std::iter::once(Ok(pattern_path)))));
        }

        let iter =
            nu_glob::dc_glob::glob_from_interruptible(cwd, &pattern, signals.interrupt_flag())
                .map_err(|e| {
                    ShellError::Generic(GenericError::new(
                        "Error extracting glob pattern",
                        e.to_string(),
                        span,
                    ))
                })?;

        // dc-glob returns paths relative to the traversal start directory.
        // Join them with `prefix` to produce absolute paths, matching the
        // legacy backend's behaviour.
        let prefix_for_map = prefix.clone();
        let mapped = iter.map(move |x| match x {
            Ok(v) => {
                let v = match &prefix_for_map {
                    Some(p) if v.is_relative() => p.join(&v),
                    _ => v,
                };
                Ok(v)
            }
            Err(e) => Err(ShellError::Generic(GenericError::new(
                "Error extracting glob pattern",
                e.to_string(),
                span,
            ))),
        });

        Ok((prefix, Box::new(mapped)))
    } else {
        let glob_options = options.unwrap_or_default();
        let glob = nu_glob::glob_with(&pattern, glob_options, signals).map_err(|e| {
            ShellError::Generic(GenericError::new(
                "Error extracting glob pattern",
                e.to_string(),
                span,
            ))
        })?;

        let mapped = glob.map(move |x| match x {
            Ok(v) => Ok(v),
            Err(e) => Err(ShellError::Generic(GenericError::new(
                "Error extracting glob pattern",
                e.error().to_string(),
                span,
            ))),
        });

        Ok((prefix, Box::new(mapped)))
    }
}

#[cfg(test)]
mod tests {
    use super::glob_from;
    use nu_protocol::{NuGlob, Signals, Span, Spanned};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        std::env::temp_dir().join(format!(
            "nu_engine_glob_from_{prefix}_{}_{}",
            std::process::id(),
            ts + u128::from(NEXT_ID.fetch_add(1, Ordering::Relaxed))
        ))
    }

    fn write_file(path: &PathBuf) {
        let create_result = fs::create_dir_all(path.parent().unwrap_or(path));
        assert!(
            create_result.is_ok(),
            "failed to create parent dir for {}: {:?}",
            path.display(),
            create_result
        );

        let write_result = fs::write(path, b"x");
        assert!(
            write_result.is_ok(),
            "failed to write test file {}: {:?}",
            path.display(),
            write_result
        );
    }

    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_remains_lazy_for_first_item() {
        let root = unique_test_dir("lazy_first_item");
        let root_create_result = fs::create_dir_all(&root);
        assert!(
            root_create_result.is_ok(),
            "failed to create root test directory {}: {:?}",
            root.display(),
            root_create_result
        );

        // A top-level match gives the iterator a fast first row.
        write_file(&root.join("top.rs"));

        // Create enough matches that eager collection would fully drain on construction.
        let nested_count = 9000usize;
        for idx in 0..nested_count {
            write_file(&root.join(format!("deep/dir_{idx}/file_{idx}.rs")));
        }

        let ctrlc = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(ctrlc);
        let pattern = Spanned {
            item: NuGlob::Expand("**/*.rs".to_string()),
            span: Span::test_data(),
        };

        let result = glob_from(&pattern, &root, Span::test_data(), None, signals.clone());
        assert!(result.is_ok(), "glob_from failed");

        let (_, mut iter) = match result {
            Ok(v) => v,
            Err(err) => panic!("glob_from failed unexpectedly: {err}"),
        };

        let first = iter.next();
        assert!(
            matches!(first, Some(Ok(_))),
            "expected first iterator item to be a match, got: {first:?}"
        );

        // Interrupt after the first row. If glob_from eagerly materializes,
        // the returned iterator has already consumed all rows and this has no effect.
        signals.trigger();

        let remaining = iter.count();
        assert!(
            remaining < 6000,
            "expected interrupt to stop iteration before full drain; remaining={remaining}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_matches_literal_file() {
        let root = unique_test_dir("literal_file");
        fs::create_dir_all(&root).expect("failed to create root");
        let file = root.join("test.txt");
        write_file(&file);

        let ctrlc = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(ctrlc);
        let pattern = Spanned {
            item: NuGlob::Expand(file.to_string_lossy().to_string()),
            span: Span::test_data(),
        };

        let result = glob_from(&pattern, Path::new("/"), Span::test_data(), None, signals);
        assert!(result.is_ok(), "glob_from failed");

        let (_, mut iter) = result.unwrap();
        let first = iter.next();
        assert!(
            matches!(first, Some(Ok(ref p)) if *p == file),
            "expected file path itself, got: {first:?}"
        );
        assert!(iter.next().is_none(), "expected exactly one result");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_matches_literal_directory() {
        let root = unique_test_dir("literal_dir");
        fs::create_dir_all(&root).expect("failed to create root");

        let ctrlc = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(ctrlc);
        let pattern = Spanned {
            item: NuGlob::Expand(root.to_string_lossy().to_string()),
            span: Span::test_data(),
        };

        let result = glob_from(&pattern, Path::new("/"), Span::test_data(), None, signals);
        assert!(result.is_ok(), "glob_from failed");

        let (_, mut iter) = result.unwrap();
        let first = iter.next();
        assert!(
            matches!(first, Some(Ok(ref p)) if *p == root),
            "expected directory path itself, got: {first:?}"
        );
        assert!(iter.next().is_none(), "expected exactly one result");

        let _ = fs::remove_dir_all(&root);
    }
}
