use nu_engine::current_dir;
use nu_engine::CallExt;
use nu_path::{expand_path_with, expand_to_real_path};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, NuPath, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::ffi::OsString;
use std::path::PathBuf;
use uu_mv::{BackupMode, UpdateMode};

#[derive(Clone)]
pub struct UMv;

impl Command for UMv {
    fn name(&self) -> &str {
        "umv"
    }

    fn usage(&self) -> &str {
        "Move files or directories."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a file",
                example: "umv before.txt after.txt",
                result: None,
            },
            Example {
                description: "Move a file into a directory",
                example: "umv test.txt my/subdirectory",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "umv *.txt my/subdirectory",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["move"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("umv")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("force", "do not prompt before overwriting", Some('f'))
            .switch("verbose", "explain what is being done.", Some('v'))
            .switch("progress", "display a progress bar", Some('p'))
            .switch("interactive", "prompt before overwriting", Some('i'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .rest(
                "paths",
                SyntaxShape::GlobPattern,
                "Rename SRC to DST, or move SRC to DIR.",
            )
            .allow_variants_without_examples(true)
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let interactive = call.has_flag(engine_state, stack, "interactive")?;
        let no_clobber = call.has_flag(engine_state, stack, "no-clobber")?;
        let progress = call.has_flag(engine_state, stack, "progress")?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let overwrite = if no_clobber {
            uu_mv::OverwriteMode::NoClobber
        } else if interactive {
            uu_mv::OverwriteMode::Interactive
        } else {
            uu_mv::OverwriteMode::Force
        };

        let cwd = current_dir(engine_state, stack)?;
        let mut paths: Vec<Spanned<NuPath>> = call.rest(engine_state, stack, 0)?;
        if paths.is_empty() {
            return Err(ShellError::GenericError {
                error: "Missing file operand".into(),
                msg: "Missing file operand".into(),
                span: Some(call.head),
                help: Some("Please provide source and destination paths".into()),
                inner: Vec::new(),
            });
        }
        if paths.len() == 1 {
            // expand path for better error message
            return Err(ShellError::GenericError {
                error: "Missing destination path".into(),
                msg: format!(
                    "Missing destination path operand after {}",
                    expand_path_with(paths[0].item.as_ref(), cwd).to_string_lossy()
                ),
                span: Some(paths[0].span),
                help: None,
                inner: Vec::new(),
            });
        }

        // Do not glob target
        let spanned_target = paths.pop().ok_or(ShellError::NushellFailedSpanned {
            msg: "Missing file operand".into(),
            label: "Missing file operand".into(),
            span: call.head,
        })?;
        let mut files: Vec<PathBuf> = Vec::new();
        for mut p in paths {
            p.item = p.item.strip_ansi_string_unlikely();
            let exp_files: Vec<Result<PathBuf, ShellError>> =
                nu_engine::glob_from(&p, &cwd, call.head, None)
                    .map(|f| f.1)?
                    .collect();
            if exp_files.is_empty() {
                return Err(ShellError::FileNotFound { span: p.span });
            };
            let mut app_vals: Vec<PathBuf> = Vec::new();
            for v in exp_files {
                match v {
                    Ok(path) => {
                        app_vals.push(path);
                    }
                    Err(e) => return Err(e),
                }
            }
            files.append(&mut app_vals);
        }

        // Make sure to send absolute paths to avoid uu_cp looking for cwd in std::env which is not
        // supported in Nushell
        for src in files.iter_mut() {
            if !src.is_absolute() {
                *src = nu_path::expand_path_with(&src, &cwd);
            }
        }

        // Add back the target after globbing
        let expanded_target = expand_to_real_path(nu_utils::strip_ansi_string_unlikely(
            spanned_target.item.to_string(),
        ));
        let abs_target_path = expand_path_with(expanded_target, &cwd);
        files.push(abs_target_path.clone());
        let files = files
            .into_iter()
            .map(|p| p.into_os_string())
            .collect::<Vec<OsString>>();
        let options = uu_mv::Options {
            overwrite,
            progress_bar: progress,
            verbose,
            suffix: String::from("~"),
            backup: BackupMode::NoBackup,
            update: UpdateMode::ReplaceAll,
            target_dir: None,
            no_target_dir: false,
            strip_slashes: false,
        };
        if let Err(error) = uu_mv::mv(&files, &options) {
            return Err(ShellError::GenericError {
                error: format!("{}", error),
                msg: format!("{}", error),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }
        Ok(PipelineData::empty())
    }
}
