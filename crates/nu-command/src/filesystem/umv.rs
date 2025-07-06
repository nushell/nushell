#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_path::expand_path_with;
use nu_protocol::{
    NuGlob,
    shell_error::{self, io::IoError},
};
use std::{ffi::OsString, path::PathBuf};
use uu_mv::{BackupMode, UpdateMode};

#[derive(Clone)]
pub struct UMv;

impl Command for UMv {
    fn name(&self) -> &str {
        "mv"
    }

    fn description(&self) -> &str {
        "Move files or directories using uutils/coreutils mv."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a file",
                example: "mv before.txt after.txt",
                result: None,
            },
            Example {
                description: "Move a file into a directory",
                example: "mv test.txt my/subdirectory",
                result: None,
            },
            Example {
                description: "Move only if source file is newer than target file",
                example: "mv -u new/test.txt old/",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "mv *.txt my/subdirectory",
                result: None,
            },
            Example {
                description: r#"Move a file into the "my" directory two levels up in the directory tree"#,
                example: "mv test.txt .../my/",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["move", "file", "files", "coreutils"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("mv")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("force", "do not prompt before overwriting", Some('f'))
            .switch("verbose", "explain what is being done.", Some('v'))
            .switch("progress", "display a progress bar", Some('p'))
            .switch("interactive", "prompt before overwriting", Some('i'))
            .switch(
                "update",
                "move and overwrite only when the SOURCE file is newer than the destination file or when the destination file is missing",
                Some('u')
            )
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .rest(
                "paths",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
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
        let update = if call.has_flag(engine_state, stack, "update")? {
            UpdateMode::ReplaceIfOlder
        } else {
            UpdateMode::ReplaceAll
        };

        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;
        let mut paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;
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
                    expand_path_with(paths[0].item.as_ref(), cwd, paths[0].item.is_expand())
                        .to_string_lossy()
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
        let mut files: Vec<(Vec<PathBuf>, bool)> = Vec::new();
        for mut p in paths {
            p.item = p.item.strip_ansi_string_unlikely();
            let exp_files: Vec<Result<PathBuf, ShellError>> =
                nu_engine::glob_from(&p, &cwd, call.head, None, engine_state.signals().clone())
                    .map(|f| f.1)?
                    .collect();
            if exp_files.is_empty() {
                return Err(ShellError::Io(IoError::new(
                    shell_error::io::ErrorKind::FileNotFound,
                    p.span,
                    PathBuf::from(p.item.to_string()),
                )));
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
            files.push((app_vals, p.item.is_expand()));
        }

        // Make sure to send absolute paths to avoid uu_cp looking for cwd in std::env which is not
        // supported in Nushell
        for (files, need_expand_tilde) in files.iter_mut() {
            for src in files.iter_mut() {
                if !src.is_absolute() {
                    *src = nu_path::expand_path_with(&*src, &cwd, *need_expand_tilde);
                }
            }
        }
        let mut files: Vec<PathBuf> = files.into_iter().flat_map(|x| x.0).collect();

        // Add back the target after globbing
        let abs_target_path = expand_path_with(
            nu_utils::strip_ansi_string_unlikely(spanned_target.item.to_string()),
            &cwd,
            matches!(spanned_target.item, NuGlob::Expand(..)),
        );
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
            update,
            target_dir: None,
            no_target_dir: false,
            strip_slashes: false,
            debug: false,
        };
        if let Err(error) = uu_mv::mv(&files, &options) {
            return Err(ShellError::GenericError {
                error: format!("{error}"),
                msg: format!("{error}"),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }
        Ok(PipelineData::empty())
    }
}
