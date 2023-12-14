use nu_cmd_base::arg_glob;
use nu_engine::{current_dir, CallExt};
use nu_glob::GlobResult;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::path::PathBuf;
use uu_cp::{BackupMode, CopyMode, UpdateMode};

// TODO: related to uucore::error::set_exit_code(EXIT_ERR)
// const EXIT_ERR: i32 = 1;

#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: &str = "/";
#[cfg(target_os = "windows")]
const PATH_SEPARATOR: &str = "\\";

#[derive(Clone)]
pub struct UCp;

impl Command for UCp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files using uutils/coreutils cp."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files", "coreutils"]
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
            .switch(
                "update",
                "copy only when the SOURCE file is newer than the destination file or when the destination file is missing",
                Some('u')
            )
            .switch("progress", "display a progress bar", Some('p'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .switch("debug", "explain how a file is copied. Implies -v", None)
            .rest("paths", SyntaxShape::Filepath, "Copy SRC file/s to DEST")
            .allow_variants_without_examples(true)
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
            Example {
                description: "Copy only if source file is newer than target file",
                example: "cp -u a b",
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
        let interactive = call.has_flag("interactive");
        let (update, copy_mode) = if call.has_flag("update") {
            (UpdateMode::ReplaceIfOlder, CopyMode::Update)
        } else {
            (UpdateMode::ReplaceAll, CopyMode::Copy)
        };
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
        let paths: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
        let mut paths: Vec<Spanned<String>> = paths
            .into_iter()
            .map(|p| Spanned {
                item: nu_utils::strip_ansi_string_unlikely(p.item),
                span: p.span,
            })
            .collect();
        if paths.is_empty() {
            return Err(ShellError::GenericError {
                error: "Missing file operand".into(),
                msg: "Missing file operand".into(),
                span: Some(call.head),
                help: Some("Please provide source and destination paths".into()),
                inner: vec![],
            });
        }

        if paths.len() == 1 {
            return Err(ShellError::GenericError {
                error: "Missing destination path".into(),
                msg: format!("Missing destination path operand after {}", paths[0].item),
                span: Some(paths[0].span),
                help: None,
                inner: vec![],
            });
        }
        let target = paths.pop().expect("Should not be reached?");
        let target_path = PathBuf::from(&target.item);
        if target.item.ends_with(PATH_SEPARATOR) && !target_path.is_dir() {
            return Err(ShellError::GenericError {
                error: "is not a directory".into(),
                msg: "is not a directory".into(),
                span: Some(target.span),
                help: None,
                inner: vec![],
            });
        };

        // paths now contains the sources

        let cwd = current_dir(engine_state, stack)?;
        let mut sources: Vec<PathBuf> = Vec::new();

        for p in paths {
            let exp_files = arg_glob(&p, &cwd)?.collect::<Vec<GlobResult>>();
            if exp_files.is_empty() {
                return Err(ShellError::FileNotFound { span: p.span });
            };
            let mut app_vals: Vec<PathBuf> = Vec::new();
            for v in exp_files {
                match v {
                    Ok(path) => {
                        if !recursive && path.is_dir() {
                            return Err(ShellError::GenericError {
                                error: "could_not_copy_directory".into(),
                                msg: "resolves to a directory (not copied)".into(),
                                span: Some(p.span),
                                help: Some(
                                    "Directories must be copied using \"--recursive\"".into(),
                                ),
                                inner: vec![],
                            });
                        };
                        app_vals.push(path)
                    }
                    Err(e) => {
                        return Err(ShellError::ErrorExpandingGlob {
                            msg: format!("error {} in path {}", e.error(), e.path().display()),
                            span: p.span,
                        });
                    }
                }
            }
            sources.append(&mut app_vals);
        }

        // Make sure to send absolute paths to avoid uu_cp looking for cwd in std::env which is not
        // supported in Nushell
        for src in sources.iter_mut() {
            if !src.is_absolute() {
                *src = nu_path::expand_path_with(&src, &cwd);
            }
        }

        let target_path = nu_path::expand_path_with(&target_path, &cwd);

        let options = uu_cp::Options {
            overwrite,
            reflink_mode,
            recursive,
            debug,
            verbose: verbose || debug,
            dereference: !recursive,
            progress_bar: progress,
            attributes_only: false,
            backup: BackupMode::NoBackup,
            copy_contents: false,
            cli_dereference: false,
            copy_mode,
            no_target_dir: false,
            one_file_system: false,
            parents: false,
            sparse_mode: uu_cp::SparseMode::Auto,
            strip_trailing_slashes: false,
            attributes: uu_cp::Attributes::NONE,
            backup_suffix: String::from("~"),
            target_dir: None,
            update,
        };

        if let Err(error) = uu_cp::copy(&sources, &target_path, &options) {
            match error {
                // code should still be EXIT_ERR as does GNU cp
                uu_cp::Error::NotAllFilesCopied => {}
                _ => {
                    return Err(ShellError::GenericError {
                        error: format!("{}", error),
                        msg: format!("{}", error),
                        span: None,
                        help: None,
                        inner: vec![],
                    })
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

        test_examples(UCp {})
    }
}
