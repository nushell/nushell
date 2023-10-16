use nu_engine::CallExt;
use nu_path::expand_to_real_path;
use nu_protocol::{
    ast::Call,
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
            return Err(ShellError::GenericError(
                "Missing file operand".into(),
                "Missing file operand".into(),
                Some(call.head),
                Some("Please provide source and destination paths".into()),
                Vec::new(),
            ));
        }

        if paths.len() == 1 {
            return Err(ShellError::GenericError(
                "Missing destination path".into(),
                format!("Missing destination path operand after {}", paths[0].item),
                Some(paths[0].span),
                None,
                Vec::new(),
            ));
        }
        let target = paths.pop().expect("Should not be reached?");
        let target_path = PathBuf::from(&target.item);
        if target.item.ends_with(PATH_SEPARATOR) && !target_path.is_dir() {
            return Err(ShellError::GenericError(
                "is not a directory".into(),
                "is not a directory".into(),
                Some(target.span),
                None,
                Vec::new(),
            ));
        };
        // paths now contains the sources
        let sources: Vec<Vec<PathBuf>> = paths
            .iter()
            .map(|p| {
                // Need to expand too make it work with globbing
                let expanded_src = expand_to_real_path(&p.item);
                match nu_glob::glob_with(&expanded_src.to_string_lossy(), GLOB_PARAMS) {
                    Ok(files) => {
                        let f = files.filter_map(Result::ok).collect::<Vec<PathBuf>>();
                        if f.is_empty() {
                            return Err(ShellError::FileNotFound(p.span));
                        }
                        let any_source_is_dir = f.iter().any(|f| matches!(f, f if f.is_dir()));
                        if any_source_is_dir && !recursive {
                            return Err(ShellError::GenericError(
                                "could_not_copy_directory".into(),
                                "resolves to a directory (not copied)".into(),
                                Some(p.span),
                                Some("Directories must be copied using \"--recursive\"".into()),
                                Vec::new(),
                            ));
                        }

                        Ok(f)
                    }
                    Err(e) => Err(ShellError::GenericError(
                        e.to_string(),
                        "invalid pattern".to_string(),
                        Some(p.span),
                        None,
                        Vec::new(),
                    )),
                }
            })
            .collect::<Result<Vec<Vec<PathBuf>>, ShellError>>()?;

        let sources = sources.into_iter().flatten().collect::<Vec<PathBuf>>();
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
            copy_mode: uu_cp::CopyMode::Copy,
            no_target_dir: false,
            one_file_system: false,
            parents: false,
            sparse_mode: uu_cp::SparseMode::Auto,
            strip_trailing_slashes: false,
            attributes: uu_cp::Attributes::NONE,
            backup_suffix: String::from("~"),
            target_dir: None,
            update: UpdateMode::ReplaceAll,
        };

        if let Err(error) = uu_cp::copy(&sources, &target_path, &options) {
            match error {
                // code should still be EXIT_ERR as does GNU cp
                uu_cp::Error::NotAllFilesCopied => {}
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

        test_examples(UCp {})
    }
}
