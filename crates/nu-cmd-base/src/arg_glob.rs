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
}
