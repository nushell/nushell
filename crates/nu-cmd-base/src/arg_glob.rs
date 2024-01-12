// utilities for expanding globs in command arguments

use nu_glob::{glob_with_parent, MatchOptions, Paths};
use nu_protocol::{ShellError, Spanned};
use std::fs;
use std::path::{Path, PathBuf};

// standard glob options to use for filesystem command arguments

const GLOB_PARAMS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

// handle an argument that could be a literal path or a glob.
// if literal path, return just that (whether user can access it or not).
// if glob, expand into matching paths, using GLOB_PARAMS options.
pub fn arg_glob(
    pattern: &Spanned<String>, // alleged path or glob
    cwd: &Path,                // current working directory
) -> Result<Paths, ShellError> {
    arg_glob_opt(pattern, cwd, GLOB_PARAMS)
}

// variant of [arg_glob] that requires literal dot prefix in pattern to match dot-prefixed path.
pub fn arg_glob_leading_dot(pattern: &Spanned<String>, cwd: &Path) -> Result<Paths, ShellError> {
    arg_glob_opt(
        pattern,
        cwd,
        MatchOptions {
            require_literal_leading_dot: true,
            ..GLOB_PARAMS
        },
    )
}

fn arg_glob_opt(
    pattern: &Spanned<String>,
    cwd: &Path,
    options: MatchOptions,
) -> Result<Paths, ShellError> {
    // remove ansi coloring (?)
    let pattern = {
        Spanned {
            item: nu_utils::strip_ansi_string_unlikely(pattern.item.clone()),
            span: pattern.span,
        }
    };

    // if there's a file with same path as the pattern, just return that.
    let pp = cwd.join(&pattern.item);
    let md = fs::metadata(pp);
    #[allow(clippy::single_match)]
    match md {
        Ok(_metadata) => {
            return Ok(Paths::single(&PathBuf::from(pattern.item), cwd));
        }
        // file not found, but also "invalid chars in file" (e.g * on Windows).  Fall through and glob
        Err(_) => {}
    }

    // user wasn't referring to a specific thing in filesystem, try to glob it.
    match glob_with_parent(&pattern.item, options, cwd) {
        Ok(p) => Ok(p),
        Err(pat_err) => Err(ShellError::InvalidGlobPattern {
            msg: pat_err.msg.into(),
            span: pattern.span,
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_glob::GlobResult;
    use nu_protocol::{Span, Spanned};
    use nu_test_support::fs::Stub::EmptyFile;
    use nu_test_support::playground::Playground;
    use rstest::rstest;

    fn spanned_string(str: &str) -> Spanned<String> {
        Spanned {
            item: str.to_string(),
            span: Span::test_data(),
        }
    }

    #[test]
    fn does_something() {
        let act = arg_glob(&spanned_string("*"), &PathBuf::from("."));
        assert!(act.is_ok());
        for f in act.expect("checked ok") {
            match f {
                Ok(p) => {
                    assert!(!p.to_str().unwrap().is_empty());
                }
                Err(e) => panic!("unexpected error {:?}", e),
            };
        }
    }

    #[test]
    fn glob_format_error() {
        let act = arg_glob(&spanned_string(r#"ab]c[def"#), &PathBuf::from("."));
        assert!(act.is_err());
    }

    #[rstest]
    #[case("*", 4, "no dirs")]
    #[case("**/*", 7, "incl dirs")]
    fn glob_subdirs(#[case] pat: &str, #[case] exp_count: usize, #[case] case: &str) {
        Playground::setup("glob_subdirs", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ]);
            sandbox.mkdir(".children");
            sandbox.within(".children").with_files(vec![
                EmptyFile("timothy.txt"),
                EmptyFile("tiffany.txt"),
                EmptyFile("trish.txt"),
            ]);

            let p: Vec<GlobResult> = arg_glob(&spanned_string(pat), &dirs.test)
                .expect("no error")
                .collect();
            assert_eq!(
                exp_count,
                p.iter().filter(|i| i.is_ok()).count(),
                " case: {case} ",
            );

            // expected behavior -- that directories are included in results (if name matches pattern)
            let t = p
                .iter()
                .any(|i| i.as_ref().unwrap().to_string_lossy().contains(".children"));
            assert!(t, "check for dir, case {case}");
        })
    }

    #[rstest]
    #[case("yehuda.txt", true, 1, "matches literal path")]
    #[case("*", false, 3, "matches glob")]
    #[case(r#"bad[glob.foo"#, true, 1, "matches literal, would be bad glob pat")]
    fn exact_vs_glob(
        #[case] pat: &str,
        #[case] exp_matches_input: bool,
        #[case] exp_count: usize,
        #[case] case: &str,
    ) {
        Playground::setup("exact_vs_glob", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("bad[glob.foo"),
            ]);

            let res = arg_glob(&spanned_string(pat), &dirs.test)
                .expect("no error")
                .collect::<Vec<GlobResult>>();

            eprintln!("res: {:?}", res);
            if exp_matches_input {
                assert_eq!(
                    exp_count,
                    res.len(),
                    " case {case}: matches input, but count not 1? "
                );
                assert_eq!(
                    &res[0].as_ref().unwrap().to_string_lossy(),
                    pat, // todo: is it OK for glob to return relative paths (not to current cwd, but to arg cwd of arg_glob)?
                );
            } else {
                assert_eq!(exp_count, res.len(), " case: {}: matched glob", case);
            }
        })
    }

    #[rstest]
    #[case(r#"realbad[glob.foo"#, true, 1, "error, bad glob")]
    fn exact_vs_bad_glob(
        // if path doesn't exist but pattern is not valid glob, should get error.
        #[case] pat: &str,
        #[case] _exp_matches_input: bool,
        #[case] _exp_count: usize,
        #[case] _tag: &str,
    ) {
        Playground::setup("exact_vs_bad_glob", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("bad[glob.foo"),
            ]);

            let res = arg_glob(&spanned_string(pat), &dirs.test);
            assert!(res
                .expect_err("expected error")
                .to_string()
                .contains("Invalid glob pattern"));
        })
    }
}
