use nu_engine::CallExt;
use nu_path::expand_to_real_path;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::ffi::OsString;
use uu_mv::{BackupMode, UpdateMode};

const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

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
                SyntaxShape::Filepath,
                "Rename SRC to DST, or move SRC to DIR",
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
        //MVPS
        // -f, --force                  do not prompt before overwriting
        // -i, --interactive            prompt before overwrite
        // v, --verbose                explain what is being done

        let interactive = call.has_flag("interactive");
        let no_clobber = call.has_flag("no-clobber");
        let progress = call.has_flag("progress");
        let verbose = call.has_flag("verbose");
        let overwrite = if no_clobber {
            uu_mv::OverwriteMode::NoClobber
        } else if interactive {
            uu_mv::OverwriteMode::Interactive
        } else {
            uu_mv::OverwriteMode::Force
        };

        let paths: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
        let paths: Vec<Spanned<String>> = paths
            .into_iter()
            .map(|p| Spanned {
                item: nu_utils::strip_ansi_string_unlikely(p.item),
                span: p.span,
            })
            .collect();
        // CHECK THIS ERROR, DONT KNOW WHAT MOVE GETS
        if paths.is_empty() {
            return Err(ShellError::GenericError(
                "Missing file operand".into(),
                "Missing file operand".into(),
                Some(call.head),
                Some("Please provide source and destination paths".into()),
                Vec::new(),
            ));
        }
        // CHECK THIS ERROR AS WELL
        if paths.len() == 1 {
            return Err(ShellError::GenericError(
                "Missing destination path".into(),
                format!("Missing destination path operand after {}", paths[0].item),
                Some(paths[0].span),
                None,
                Vec::new(),
            ));
        }

        // Do not glob target
        let sources = &paths[..paths.len() - 1];
        let sources: Vec<Vec<OsString>> = sources
            .iter()
            .map(|p| {
                // Need to expand too make it work with globbing
                let expanded_src = expand_to_real_path(&p.item);
                match nu_glob::glob_with(&expanded_src.to_string_lossy(), GLOB_PARAMS) {
                    Ok(files) => {
                        // let f = files.filter_map(Result::ok).collect::<Vec<PathBuf>>();
                        let f = files
                            .filter_map(Result::ok)
                            .map(|p| p.into())
                            .collect::<Vec<OsString>>();
                        if f.is_empty() {
                            return Err(ShellError::FileNotFound(p.span));
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
            .collect::<Result<Vec<Vec<OsString>>, ShellError>>()?;

        let mut files = sources.into_iter().flatten().collect::<Vec<OsString>>();
        // Add back the target after globbing
        let target = paths.last().expect("Should not be reached");
        files.push(OsString::from(target.item.clone()));
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
            return Err(ShellError::GenericError(
                format!("{}", error),
                format!("{}", error),
                None,
                None,
                Vec::new(),
            ));
        }
        Ok(PipelineData::empty())
    }
}
