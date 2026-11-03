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
///
/// With `require_literal_leading_dot` set in `options`, a glob matches a name that starts with
/// `.` only where the pattern spells that dot out, at any depth, on either glob backend. See
/// [`HiddenNameFilter`].
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
    let skip_hidden = options.is_some_and(|options| options.require_literal_leading_dot);
    let mut hidden_name_filter = None;
    let (prefix, pattern) = if nu_glob::is_glob_with_backend(pattern.item.as_ref()) {
        // Pattern contains glob, split it
        let mut p = PathBuf::new();
        let path = PathBuf::from(&pattern.item.as_ref());
        let components = path.components();
        let mut counter = 0;

        // A component with pattern syntax ends the literal prefix. With dc-glob that includes
        // `{`, so `{src,tests}` in `{src,tests}/**/*.rs` is matched as alternatives.
        for c in components {
            if let Component::Normal(os) = c
                && nu_glob::is_glob_component_with_backend(os.to_string_lossy().as_ref())
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

        if skip_hidden && !no_glob_for_pattern {
            hidden_name_filter = Some(HiddenNameFilter::new(path.clone(), &just_pattern));
        }

        (Some(path), escaped_prefix.join(just_pattern))
    } else {
        let path = PathBuf::from(&pattern.item.as_ref());
        let path = expand_path_with(path, cwd, pattern.item.is_expand());
        let is_symlink = match fs::symlink_metadata(&path) {
            Ok(attr) => attr.file_type().is_symlink(),
            Err(_) => false,
        };

        if is_symlink {
            // Escaped for the same reason as an existing path below.
            let pattern = PathBuf::from(nu_glob::escape_with_backend(&path.to_string_lossy()));
            (path.parent().map(|parent| parent.to_path_buf()), pattern)
        } else {
            let path = match absolute_with(path.clone(), cwd) {
                // Our path might contain glob metacharacters too, or braces that dc-glob would
                // read as alternatives, so escape it to make glob match it literally. Escaping
                // leaves a path without them unchanged.
                Ok(p) if p.exists() => {
                    PathBuf::from(nu_glob::escape_with_backend(&p.to_string_lossy()))
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
        // The dc-glob traversal handles literal paths itself: a pattern that
        // names an existing path matches that path, and a pattern that still
        // contains glob metacharacters keeps going through the walker even
        // when a same-named path exists (e.g. a file named `*` must not make
        // bare `*` / `ls` return only that one entry). See #18631.
        //
        // The caller's match options carry over, so hidden names are skipped the same way the
        // legacy backend skips them (`rm *` must not reach dotfiles).
        let walk_options = nu_glob::dc_glob::GlobWalkOptions {
            interrupt: signals.interrupt_flag(),
            ..nu_glob::dc_glob::GlobWalkOptions::nu_glob(options.unwrap_or_default())
        };
        let iter = nu_glob::dc_glob::glob_with(cwd, &pattern, &walk_options).map_err(|e| {
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

        Ok((prefix, keep_allowed(mapped, hidden_name_filter)))
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

        Ok((prefix, keep_allowed(mapped, hidden_name_filter)))
    }
}

/// Boxes a glob's `matches`, dropping the ones `filter` doesn't allow. Errors are kept.
fn keep_allowed(
    matches: impl Iterator<Item = Result<PathBuf, ShellError>> + Send + 'static,
    filter: Option<HiddenNameFilter>,
) -> Box<dyn Iterator<Item = Result<PathBuf, ShellError>> + Send> {
    match filter {
        Some(filter) => Box::new(
            matches.filter(move |path| path.as_ref().map_or(true, |path| filter.allows(path))),
        ),
        None => Box::new(matches),
    }
}

/// Drops a glob's matches that reach a name starting with `.` where the pattern doesn't spell
/// that dot out, at any depth: `*`, `?`, `[...]` and `**` never match a hidden name on their own.
/// `.*` and `src/.cache/*` spell the dot out; so does the literal part of the pattern, the
/// directory the glob starts from, so `.cache/*` and a glob run inside a hidden directory work.
///
/// Both backends already hold `*`, `?` and `[...]` to this through
/// `require_literal_leading_dot`, but their `**` walks into hidden directories, so `glob_from`
/// checks every match here instead.
///
/// Only a whole `**` part counts as recursive. A dc-glob part like `{.a,b}` counts as one name
/// that doesn't spell a dot, so its hidden matches are dropped rather than kept.
struct HiddenNameFilter {
    /// The directory the glob starts from.
    start: PathBuf,
    /// The number of components in `start`.
    start_len: usize,
    /// The pattern's parts below `start`.
    parts: Vec<GlobPart>,
}

/// A part of a glob pattern, as far as hidden names go.
#[derive(Clone, Copy)]
enum GlobPart {
    /// `**`: any number of names, none of them hidden.
    Recurse,
    /// One name, hidden exactly when the part starts with `.`.
    One { hidden: bool },
}

impl HiddenNameFilter {
    fn new(start: PathBuf, pattern: &Path) -> Self {
        let parts = pattern
            .components()
            .map(|part| match part {
                Component::Normal(name) if name == "**" => GlobPart::Recurse,
                part => GlobPart::One {
                    hidden: is_hidden(part),
                },
            })
            .collect();
        Self {
            start_len: start.components().count(),
            start,
            parts,
        }
    }

    /// Whether the match `path` may be kept.
    fn allows(&self, path: &Path) -> bool {
        // Both backends build every match on `start`, so most matches, the ones with no hidden
        // name below it, are settled by skipping its components, without re-parsing it.
        debug_assert!(
            path.starts_with(&self.start),
            "{path:?} isn't a match below {:?}",
            self.start
        );
        if !path.components().skip(self.start_len).any(is_hidden) {
            return true;
        }
        // Should a match turn up outside `start` anyway, drop it rather than guess.
        let Ok(below) = path.strip_prefix(&self.start) else {
            return false;
        };
        let names: Vec<bool> = below.components().map(is_hidden).collect();
        dots_spelled_out(&self.parts, &names)
    }
}

/// Whether a component is a name that starts with `.`. `.` and `..` themselves aren't names.
fn is_hidden(component: Component) -> bool {
    matches!(component, Component::Normal(name) if name.as_encoded_bytes().first() == Some(&b'.'))
}

/// Whether the pattern `parts` can stand for a match whose names below the start directory are
/// hidden exactly where `names` is `true`.
fn dots_spelled_out(parts: &[GlobPart], names: &[bool]) -> bool {
    // `reach[j]`: the parts so far can stand for exactly the first `j` names.
    let mut reach = vec![false; names.len() + 1];
    reach[0] = true;
    for part in parts {
        match *part {
            GlobPart::Recurse => {
                for j in 1..=names.len() {
                    reach[j] |= reach[j - 1] && !names[j - 1];
                }
            }
            GlobPart::One { hidden } => {
                for j in (1..=names.len()).rev() {
                    reach[j] = reach[j - 1] && names[j - 1] == hidden;
                }
                reach[0] = false;
            }
        }
    }
    reach[names.len()]
}

#[cfg(test)]
mod tests {
    use super::{HiddenNameFilter, glob_from};
    use nu_glob::MatchOptions;
    use nu_protocol::{NuGlob, Signals, Span, Spanned};
    use rstest::rstest;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    /// Which matches of a pattern, run from `/s`, keep a hidden name.
    #[rstest]
    #[case::star_plain("*", "foo", true)]
    #[case::star_hidden("*", ".bar", false)]
    #[case::dot_star("*.txt", ".b.txt", false)]
    #[case::dot_spelled(".*", ".bar", true)]
    #[case::recurse_plain("**/*.txt", "d/x.txt", true)]
    #[case::recurse_into_hidden("**/*.txt", ".d/x.txt", false)]
    #[case::recurse_into_nested_hidden("**/*.txt", "d/.e/x.txt", false)]
    #[case::recurse_hidden_leaf("**/*.txt", "d/.x.txt", false)]
    #[case::recurse_alone_is_start("**", "", true)]
    #[case::recurse_alone_hidden("**", "d/.e", false)]
    #[case::spelled_after_star("*/.cache", "d/.cache", true)]
    #[case::star_before_spelled("*/.cache", ".v/.cache", false)]
    #[case::recurse_to_spelled("**/.DS_Store", ".DS_Store", true)]
    #[case::recurse_to_spelled_nested("**/.DS_Store", "d/.DS_Store", true)]
    #[case::recurse_through_hidden("**/.DS_Store", ".git/.DS_Store", false)]
    #[case::spelled_between_recurses("**/.cache/**", "a/.cache/b", true)]
    #[case::hidden_before_spelled("**/.cache/**", ".a/.cache/b", false)]
    #[case::hidden_after_spelled("**/.cache/**", "a/.cache/.b", false)]
    #[case::star_after_spelled("d/.git/*", "d/.git/.x", false)]
    #[case::parent_dir("d*/../*", "d1/../x", true)]
    #[case::parent_dir_hidden("d*/../*", "d1/../.h", false)]
    fn hidden_name_filter(#[case] pattern: &str, #[case] below: &str, #[case] kept: bool) {
        let filter = HiddenNameFilter::new(PathBuf::from("/s"), Path::new(pattern));
        assert_eq!(
            filter.allows(&Path::new("/s").join(below)),
            kept,
            "`{pattern}` matching `{below}`"
        );
    }

    /// The directory a glob starts from spells its own dots out, even outside the pattern.
    #[test]
    fn hidden_name_filter_ignores_hidden_start() {
        let filter = HiddenNameFilter::new(PathBuf::from("/h/.config/nu"), Path::new("*"));
        assert!(filter.allows(Path::new("/h/.config/nu/x")));
        assert!(!filter.allows(Path::new("/h/.config/nu/.y")));
    }

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

    // Windows does not allow `*` in filenames, so this regression only applies on Unix.
    #[cfg(not(windows))]
    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_star_with_literal_star_file() {
        // Regression for #18631: a file named `*` must not make pattern `*`
        // short-circuit to only that path.
        let root = unique_test_dir("star_file");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("a"));
        write_file(&root.join("b"));
        write_file(&root.join("*"));

        let ctrlc = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(ctrlc);
        let pattern = Spanned {
            item: NuGlob::Expand("*".to_string()),
            span: Span::test_data(),
        };

        let result = glob_from(&pattern, &root, Span::test_data(), None, signals);
        assert!(result.is_ok(), "glob_from failed");

        let (_, iter) = match result {
            Ok(v) => v,
            Err(err) => panic!("glob_from failed unexpectedly: {err}"),
        };
        let mut names: Vec<String> = iter
            .map(|r| {
                r.expect("glob path ok")
                    .file_name()
                    .expect("basename")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();

        assert_eq!(
            names,
            vec!["*".to_string(), "a".to_string(), "b".to_string()]
        );

        let _ = fs::remove_dir_all(&root);
    }

    // Windows does not allow `*` in filenames, so this regression only applies on Unix.
    #[cfg(not(windows))]
    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_prefix_wildcard_with_literal_match_name() {
        // Pattern `foo*` must still expand when a file literally named `foo*` exists.
        let root = unique_test_dir("foo_star");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("foo1"));
        write_file(&root.join("foo2"));
        write_file(&root.join("foo*"));
        write_file(&root.join("other"));

        let ctrlc = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(ctrlc);
        let pattern = Spanned {
            item: NuGlob::Expand("foo*".to_string()),
            span: Span::test_data(),
        };

        let result = glob_from(&pattern, &root, Span::test_data(), None, signals);
        assert!(result.is_ok(), "glob_from failed");

        let (_, iter) = match result {
            Ok(v) => v,
            Err(err) => panic!("glob_from failed unexpectedly: {err}"),
        };
        let mut names: Vec<String> = iter
            .map(|r| {
                r.expect("glob path ok")
                    .file_name()
                    .expect("basename")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();

        assert_eq!(
            names,
            vec!["foo*".to_string(), "foo1".to_string(), "foo2".to_string()]
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// Collect what `glob_from` yields for `pattern` under `root`, as paths relative to `root`.
    fn glob_relative(root: &Path, pattern: &str, options: Option<MatchOptions>) -> Vec<PathBuf> {
        let pattern = Spanned {
            item: NuGlob::Expand(pattern.to_string()),
            span: Span::test_data(),
        };
        let (_, iter) = glob_from(&pattern, root, Span::test_data(), options, Signals::empty())
            .expect("glob_from should succeed");
        let mut paths: Vec<PathBuf> = iter
            .map(|path| {
                let path = path.expect("glob entry");
                path.strip_prefix(root).unwrap_or(&path).to_path_buf()
            })
            .collect();
        paths.sort();
        paths
    }

    /// A trailing `**` never yields the directory it starts from, as with nu-glob, so
    /// `rm -r **` can't reach the current directory.
    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_double_star_skips_its_start() {
        let root = unique_test_dir("double_star_start");
        write_file(&root.join("a/b/file.txt"));

        assert_eq!(
            glob_relative(&root, "**", None),
            [PathBuf::from("a"), PathBuf::from("a/b")]
        );
        assert_eq!(glob_relative(&root, "a/**", None), [PathBuf::from("a/b")]);

        let _ = fs::remove_dir_all(&root);
    }

    /// The caller's `require_literal_leading_dot` reaches dc-glob, so `rm *` keeps dotfiles.
    #[test]
    #[exp(nu_experimental::DC_GLOB)]
    fn glob_from_dc_glob_honors_literal_leading_dot() {
        let root = unique_test_dir("literal_leading_dot");
        write_file(&root.join("visible"));
        write_file(&root.join(".hidden"));
        let options = MatchOptions {
            require_literal_leading_dot: true,
            ..MatchOptions::default()
        };

        assert_eq!(
            glob_relative(&root, "*", Some(options)),
            [PathBuf::from("visible")]
        );
        assert_eq!(
            glob_relative(&root, ".*", Some(options)),
            [PathBuf::from(".hidden")]
        );

        let _ = fs::remove_dir_all(&root);
    }
}
