// utilities for expanding globs in command arguments
use nu_protocol::{ShellError, Spanned};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use wax::{Glob, LinkBehavior, WalkBehavior};

// standard glob options to use for filesystem command arguments
const FS_CMD_WALK_BEHAVIOR: WalkBehavior = WalkBehavior {
    depth: usize::MAX,
    link: LinkBehavior::ReadTarget,
};

// handle an argument that could be a literal path or a glob.
// if literal path, return just that (whether user can access it or not).
// if glob, expand into matching paths, using FS_CMD_WALK_BEHAVIOR
//todo: see about returning WalkError or BuildError, or enabling miette and dealing with diagnostic
pub fn arg_glob(
    pattern: &Spanned<String>, // alleged path or glob
    include_dirs: bool,        // include dirs in results.  Default (f) only include files
    cwd: &PathBuf,             // current working directory
) -> Result<Vec<PathBuf>, ShellError> {
    // stat the path first, return path if not not found.
    match fs::metadata(&pattern.item) {
        Ok(_metadata) => {
            let normalized_path = nu_path::canonicalize_with(&pattern.item, cwd)?;
            return Ok(vec![normalized_path]);
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // fall through and try the glob
        }
        Err(_) => {
            // no access, invalid chars in file, anything else: there was something at that path, return it to caller
            let normalized_path = nu_path::canonicalize_with(&pattern.item, cwd)?;
            return Ok(vec![normalized_path]);
        }
    }

    // user wasn't referring to a specific thing in filesystem, try to glob it.
    let (prefix, glob) = match Glob::new(&pattern.item) {
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

#[cfg(test)]
mod test {
    use super::*;
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
        let act = arg_glob(&spanned_string("*"), true, &PathBuf::from("."));
        assert!(act.is_ok());
        assert!(!act.expect("was OK").is_empty())
    }

    #[test]
    fn glob_format_error() {
        let act = arg_glob(&spanned_string(r#"ab{c]def"#), false, &PathBuf::from("."));
        assert!(act.is_err());
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

            let res = arg_glob(&spanned_string(pat), with_dirs, &dirs.test).expect("no error");

            assert_eq!(exp_count, res.len(), "Expected : Actual");
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
        #[case] _tag: &str,
    ) {
        Playground::setup("exact_vs_glob", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("bad[glob.foo"),
            ]);

            // Playground doesn't actually change process cwd, which fs::metadata is going to depend on
            // so we make the input an absolute path, so at least arg_glob() will stat the right thing.  Good enough for unit test.
            let abs_pat = dirs.test.join(pat).to_string_lossy().to_string();

            let res = arg_glob(&spanned_string(&abs_pat), true, &dirs.test).expect("no error");

            if exp_matches_input {
                assert_eq!(exp_count, res.len(), "matches input, but count not 1? ");
                assert_eq!(nu_path::canonicalize_with(pat, dirs.test).expect("canonicalize_with?"), res[0])

            } else {
                assert_eq!(exp_count, res.len(), "Expected : Actual");
            }
        })
    }

    #[rstest]
    #[case(r#"realbad[glob.foo"#, true, 1, "error, bad glob")]
    fn exact_vs_glob_bad_glob(      // if path doesn't exist but pattern is not valid glob, should get error.
        #[case] pat: &str,
        #[case] _exp_matches_input: bool,
        #[case] _exp_count: usize,
        #[case] _tag: &str,
    ) {
        Playground::setup("exact_vs_glob_bad_glob", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("bad[glob.foo"),
            ]);

            // Playground doesn't actually change process cwd, which fs::metadata is going to depend on
            // so we make the input an absolute path, so at least arg_glob() will stat the right thing.  Good enough for unit test.
            let abs_pat = dirs.test.join(pat).to_string_lossy().to_string();

            let res = arg_glob(&spanned_string(&abs_pat), true, &dirs.test).expect_err("no error");
            assert!(res.to_string().contains("Invalid glob pattern"));
        })
    }
}
