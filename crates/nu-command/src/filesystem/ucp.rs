use nu_engine::{env::current_dir, CallExt};
use nu_protocol::{
    ast::{Argument, Call},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::path::PathBuf;
use uu_cp::{BackupMode, UpdateMode};

// TODO: related to uucore::error::set_exit_code(EXIT_ERR)
// const EXIT_ERR: i32 = 1;
const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

/// Returns tuple of (Source paths, Target)
pub fn parse_path_args(
    mut paths: Vec<PathBuf>,
    options: &uu_cp::Options,
) -> Result<(Vec<PathBuf>, PathBuf), ShellError> {
    if paths.is_empty() {
        // No files specified
        return Err(ShellError::GenericError(
            "Missing file operand".into(),
            "".into(),
            None,
            None,
            Vec::new(),
        ));
        // return Err("missing file operand".into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && options.target_dir.is_none() && paths.len() > 2 {
        // FIX THIS ERROR PROPERLY
        return Err(ShellError::GenericError(
            "extra operand".into(),
            "".into(),
            None,
            None,
            Vec::new(),
        ));
        // return Err(format!("extra operand {:?}", paths[2]).into());
    }

    let target = match options.target_dir {
        Some(ref target) => {
            // All path args are sources, and the target dir was
            // specified separately
            target.clone()
        }
        None => {
            // If there was no explicit target-dir, then use the last
            // path_arg
            paths.pop().unwrap()
        }
    };

    // See comments on strip_trailing_slashes
    // if options.strip_trailing_slashes {
    //     for source in &mut paths {
    //         *source = source.components().as_path().to_owned();
    //     }
    // }

    Ok((paths, target))
}
#[derive(Clone)]
pub struct Ucp;

impl Command for Ucp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files using uutils/coreutils cp."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("recursive", "copy directories recursively", Some('r'))
            .switch("verbose", "explicitly state what is being done", Some('v'))
            .switch(
                "force",
                "if an existing destination file cannot be opened, remove it and try
                    again (this option is ignored when the -n option is also used).
                    currently not implemented for windows",
                Some('f'),
            )
            .switch("interactive", "ask before overwriting files", Some('i'))
            .switch("progress", "display a progress bar", Some('p'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .switch("debug", "explain how a file is copied. Implies -v", None)
            .rest("paths", SyntaxShape::Filepath, "the place to copy to")
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Copy myfile to dir_b",
                example: "cp myfile dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b",
                example: "cp -r dir_a dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b, and print the feedbacks",
                example: "cp -r -v dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "cp *.txt dir_a",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // CRAWL defined args

        // [OPTIONS] SOURCE DEST
        // -r, --recursive     copy directories recursively [short aliases: R]
        // -v, --verbose       explicitly state what is being done (also adds --debug)
        // -f, --force         if an existing destination file cannot be opened, remove it and try again
        //                     (this option is ignored when the -n option is also used). Currently not
        //                     implemented for Windows.
        // -i, --interactive   ask before overwriting files
        // -g, --progress      Display a progress bar.
        // -n, --no-clobber    do not overwrite an existing file (overrides a previous -i option)
        // None, --debug       explain how a file is copied. Implies -v.
        let interactive = call.has_flag("interactive");
        let force = call.has_flag("force");
        let no_clobber = call.has_flag("no-clobber");
        let progress = call.has_flag("progress");
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");

        let debug = call.has_flag("debug");
        let overwrite = if no_clobber {
            uu_cp::OverwriteMode::NoClobber
        } else if interactive {
            if force {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Force)
            } else {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Standard)
            }
        } else if force {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Force)
        } else {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Standard)
        };
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
        let reflink_mode = uu_cp::ReflinkMode::Auto;
        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
        let reflink_mode = uu_cp::ReflinkMode::Never;

        //Conflicting flags need to be handled. For now lets handle the ones that I can see
        //for now with uutils tests.
        let current_dir_path = current_dir(engine_state, stack)?;
        let num_files = call
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional(expr) => Some(expr),
                _ => None,
            })
            .count();

        let mut paths: Vec<PathBuf> = vec![];
        for file_nr in 0..num_files {
            let path = call.req::<Spanned<String>>(engine_state, stack, file_nr)?;
            let mut path = {
                Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path.item),
                    span: path.span,
                }
            };
            // this is the target
            if file_nr == num_files - 1 {
                // IF only one source and one target, and if target is a directory
                let target = PathBuf::from(&path.item);
                if path.item.ends_with('/')
                    && !target.is_dir()
                    && !call.has_flag("target-directory")
                {
                    return Err(ShellError::GenericError(
                        "is not a directory".into(),
                        "is not a directory".into(),
                        Some(path.span),
                        None,
                        Vec::new(),
                    ));
                };
                paths.push(target);
            } else {
                // these are the sources
                let src = &mut path.item;
                match nu_glob::glob_with(src, GLOB_PARAMS) {
                    Ok(files) => {
                        let mut f = files.filter_map(Result::ok).collect::<Vec<PathBuf>>();
                        if f.is_empty() {
                            return Err(ShellError::FileNotFound(path.span));
                        }
                        paths.append(&mut f);
                    }
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            e.to_string(),
                            "invalid pattern".to_string(),
                            Some(path.span),
                            None,
                            Vec::new(),
                        ))
                    }
                }
            }
        }
        let options = uu_cp::Options {
            attributes_only: false,
            backup: BackupMode::NoBackup,
            copy_contents: false,
            cli_dereference: false,
            copy_mode: uu_cp::CopyMode::Copy,
            // dereference,
            dereference: !recursive,
            no_target_dir: false,
            one_file_system: false,
            overwrite,
            parents: false,
            sparse_mode: uu_cp::SparseMode::Auto,
            strip_trailing_slashes: false,
            reflink_mode,
            attributes: uu_cp::Attributes::NONE,
            recursive,
            backup_suffix: String::from("~"), // default
            target_dir: None,
            update: UpdateMode::ReplaceAll,
            debug,
            verbose: verbose || debug,
            progress_bar: progress,
        };

        // For enabling easy `crawl` of cp command, we need to strip current directory
        // as it looks like uu_cp takes relative directory for sources & target,
        // but nushell always resolves to the $CWD
        // again, kind of a hack
        let paths = paths
            .iter()
            .map(|path| match path.strip_prefix(&current_dir_path) {
                Ok(p) => {
                    if p.to_str()?.is_empty() {
                        Some(PathBuf::from("."))
                    } else {
                        Some(p.to_path_buf())
                    }
                }
                Err(_) => Some(path.to_path_buf()),
            })
            // I guess this is caught by nu, if not
            // we handle it here.
            .collect::<Option<Vec<PathBuf>>>()
            .ok_or(ShellError::GenericError(
                "Not valid UTF-8".to_string(),
                "Not valid UTF-8".to_string(),
                None,
                None,
                vec![],
            ))?;
        // This function below was taken directly from `uu_cp` to reuse
        // some of their logic when it came to parsing path arguments,
        // even tho we already did some of it in the `src, target` for loop above
        // Nice to combine efforts and make it nicer.

        // TODO: Fix this following the comments in draft PR by Terts
        let (sources, target) = parse_path_args(paths, &options)?;
        if let Err(error) = uu_cp::copy(&sources, &target, &options) {
            match error {
                // Error::NotAllFilesCopied is non-fatal, but the error
                // code should still be EXIT_ERR as does GNU cp
                uu_cp::Error::NotAllFilesCopied => {}
                // Else we caught a fatal bubbled-up error, log it to stderr
                // ShellError
                // _ => uucore::macros::show_error!("{}", error),
                _ => {
                    return Err(ShellError::GenericError(
                        format!("{}", error),
                        format!("{}", error),
                        None,
                        None,
                        Vec::new(),
                    ))
                }
            };
            // TODO: What should we do in place of set_exit_code?
            // uucore::error::set_exit_code(EXIT_ERR);
        }
        Ok(PipelineData::empty())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Ucp {})
    }
}
