// utilities for expanding globs in command arguments
use nu_protocol::{ShellError, Spanned};
use std::path::PathBuf;
use wax::{Glob, LinkBehavior, WalkBehavior};

// standard glob options to use for filesystem command arguments
const FS_CMD_WALK_BEHAVIOR: WalkBehavior = WalkBehavior {
    depth: usize::MAX,
    link: LinkBehavior::ReadTarget,
};

// expand a glob into matching paths, using FS_CMD_WALK_BEHAVIOR
//todo: see about returning WalkError or BuildError, or enabling miette and dealing with diagnostic
pub fn arg_glob(
    pattern: &Spanned<String>, // the glob
    include_dirs: bool,        // include dirs in results.  Default (f) only include files
    cwd: &PathBuf,             // current working directory
) -> Result<Vec<PathBuf>, ShellError> {
    #[cfg(windows)]
    let processed_item = &windows_pattern_hack(pattern.item);
    #[cfg(not(windows))]
    let processed_item = &pattern.item;

    let (prefix, glob) = match Glob::new(processed_item) {
        Ok(p) => p.partition(),
        Err(e) => {
            return Err(ShellError::InvalidGlobPattern(e.to_string(), pattern.span));
        }
    };

    let path = match nu_path::canonicalize_with(prefix, cwd) {
        Ok(path) => path,
        Err(e) if e.to_string().contains("os error 2") =>
        // path we're trying to glob doesn't exist,
        {
            PathBuf::new() // ensure glob doesn't match anything
        }
        Err(e) => return Err(ShellError::ErrorExpandingGlob(format!("{e}"), pattern.span)),
    };

    let mut rv: Vec<PathBuf> = Vec::new();
    for w in glob
        .walk_with_behavior(path, FS_CMD_WALK_BEHAVIOR)
        .flatten()
        .filter(|w| (include_dirs && w.file_type().is_dir()) || w.file_type().is_file())
    {
        rv.push(w.path().to_path_buf())
    }
    Ok(rv)
}

#[cfg(any(windows, test))]
// Sanitize a glob pattern for use on windows.
// this allows filesystem commands like `cp` to accept an arg that can be a path or a glob.
// Deal with 3 aspects of windows paths that can confound glob patterns:
// * path separator: change `\` to `/`. Rust file API on windows supports this in folder separators and in UNC paths.
// * drive letter colon: change `C:` to `C\:`
// * quote parens (such as the infamous `Program Files (x86)`).
// But this means user cannot use `\` to quote other metachars.
// (Windows) users of glob should adopt the habit of quoting with single char classes, e.g instead of `\*`, use `[*]`.
// e.g transform something like: `C:\Users\me\.../{abc,def}*.txt` to `C\:/Users/me/.../{abc,def}*.txt`
//todo: investigate enhancing [wax] to support this more elegantly.
pub fn windows_pattern_hack(pat: &str) -> String {
    pat.replace('\\', "/")
        .replace(r#":/"#, r#"\:/"#) // must come after above
        .replace('(', r#"\("#) // when path includes Program Files (x86)
        .replace(')', r#"\)"#)
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::{Span, Spanned};
    use nu_test_support::fs::Stub::EmptyFile;
    use nu_test_support::playground::Playground;
    use rstest::rstest;

    fn test_glob(pat: &str) -> Spanned<String> {
        Spanned {
            item: pat.to_string(),
            span: Span::unknown(),
        }
    }

    #[test]
    fn does_something() {
        let act = arg_glob(&test_glob("*"), true, &PathBuf::from("."));
        assert!(act.is_ok());
        assert!(!act.expect("was OK").is_empty())
    }

    #[test]
    fn glob_format_error() {
        let act = arg_glob(&test_glob(r#"ab{c]def"#), false, &PathBuf::from("."));
        assert!(act.is_err());
        println!("act was {:#?}", act);
    }

    #[rstest]
    #[case("*", false, 3, "no dirs")]
    #[case("*", true, 4, "incl dirs")]
    fn glob_subdirs(
        #[case] pat: &str,
        #[case] with_dirs: bool,
        #[case] exp_count: usize,
        #[case] _tag: &str,
    ) {
        Playground::setup("glob_subdirs", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ]);
            sandbox.mkdir("children");
            sandbox.within("children").with_files(vec![
                EmptyFile("timothy.txt"),
                EmptyFile("tiffany.txt"),
                EmptyFile("trish.txt"),
            ]);

            let res = arg_glob(&test_glob(pat), with_dirs, &dirs.test).expect("no error");

            assert_eq!(exp_count, res.len(), "Expected : Actual");
        })
    }

    #[rstest]
    #[case("**/*.{abc,def}", "**/*.{abc,def}")]
    #[case(r#"/c/d/e"#, r#"/c/d/e"#)]
    #[case(r#"\Users\me\mine.txt"#, r#"/Users/me/mine.txt"#)]
    #[case(
        r#"C:\Program Files (x86)\nushell\bin\nu"#,
        r#"C\:/Program Files \(x86\)/nushell/bin/nu"#
    )]
    #[ignore = "should work on windows, check later"]
    #[case(
        r#"\\localhost\shareme\foo\bar/**"#,
        r#"//localhost/shareme/foo/bar/**"#
    )]

    fn test_windows_pattern_hack_works(#[case] pat: &str, #[case] expected: &str) {
        let act_pat = windows_pattern_hack(pat);
        assert_eq!(expected, act_pat);

        match Glob::new(&act_pat) {
            Ok(_) => {}
            Err(e) => {
                /*for d in e.locations() {
                    let (start, n) = e.span();
                    let fragment = &act_pat[start..][..n];
                    eprintln!("in sub-expression `{}`: {}", fragment, e);
                }*/
                panic!("glob err: {}", e);
            }
        }
    }

    // highlight the cases where the hack breaks the glob and the user must devise alternate quoting.
    #[rstest]
    #[case(r#"C:xyzzy"#, r#"C[:]xyzzy"#, r#"C:xyzzy"#)]

    fn test_windows_pattern_hack_fails(
        #[case] failing_pat: &str,
        #[case] working_pat: &str,
        #[case] _expected: &str,
    ) {
        let act_pat = windows_pattern_hack(failing_pat);
        // the hacked pattern might not give the desired output pattern to feed to glob.
        // or, even if it does, that pattern won't parse.

        assert!(Glob::new(&act_pat).is_err());

        let act_pat = windows_pattern_hack(working_pat);
        //no! assert_eq!(expected, act_pat);
        // but we claim that the pattern globs to the right thing.  Check here that it at least parses.

        match Glob::new(&act_pat) {
            Ok(_) => {}
            Err(e) => {
                /*for d in e.locations() {
                    let (start, n) = e.span();
                    let fragment = &act_pat[start..][..n];
                    eprintln!("in sub-expression `{}`: {}", fragment, e);
                }*/
                panic!("glob err: {}", e);
            }
        }
    }
}
