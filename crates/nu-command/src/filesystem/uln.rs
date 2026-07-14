use nu_engine::command_prelude::*;
use nu_protocol::{NuGlob, shell_error::generic::GenericError};

use uu_ln::{LnError, OverwriteMode, Settings};
use uucore::backup_control::BackupMode;
use uucore::{localized_help_template, translate};

use std::{ffi::OsString, path::PathBuf};

#[derive(Clone)]
pub struct ULn;

impl Command for ULn {
    fn name(&self) -> &str {
        "ln"
    }

    fn description(&self) -> &str {
        "Make links between files using uutils/coreutils ln."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["link", "file", "files", "coreutils"]
    }
    fn signature(&self) -> Signature {
        Signature::build("ln")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("force", "Remove existing destination files", Some('f'))
            .switch("verbose", "Print name of each linked file.", Some('v'))
            .switch(
                "symbolic",
                "Make symbolic links instead of hard links.",
                Some('s'),
            )
            .switch(
                "relative",
                "With -s, create links relative to link location.",
                Some('r'),
            )
            .switch(
                "logical",
                "Dereference TARGETs that are symbolic links",
                Some('L'),
            )
            .switch(
                "no-dereference",
                "Treat LINK_NAME as a normal file if it is a symbolic link to a directory.",
                Some('n'),
            )
            .switch(
                "no-target-directory",
                "Treat LINK_NAME as a normal file always",
                Some('T'),
            )
            .switch(
                "interactive",
                "Prompt whether to remove destinations.",
                Some('i'),
            )
            .rest(
                "paths",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
                "Create a link to TARGET.",
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
        // setup the uutils error translation
        let _ = localized_help_template("ln");
        let force = call.has_flag(engine_state, stack, "force")?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let symbolic = call.has_flag(engine_state, stack, "symbolic")?;
        let relative = call.has_flag(engine_state, stack, "relative")?;
        let logical = call.has_flag(engine_state, stack, "logical")?;
        let no_dereference = call.has_flag(engine_state, stack, "no-dereference")?;
        let no_target_dir = call.has_flag(engine_state, stack, "no-target-directory")?;
        let interactive = call.has_flag(engine_state, stack, "interactive")?;
        let spanned_paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;

        if spanned_paths.is_empty() {
            return Err(ShellError::Generic(
                GenericError::new("Missing file", "Missing file", call.head)
                    .with_help("Please provide at least a file"),
            ));
        }

        if relative && !symbolic {
            return Err(ShellError::Generic(
                GenericError::new(
                    "Missing required argument",
                    "symbolic argument is required",
                    call.head,
                )
                .with_help("Add symbolic argument when using relative argument"),
            ));
        }

        let overwrite_mode = if force {
            OverwriteMode::Force
        } else if interactive {
            OverwriteMode::Interactive
        } else {
            OverwriteMode::NoClobber
        };

        let cwd = engine_state.cwd(Some(stack))?.into_std_path_buf();

        let paths: Vec<PathBuf> = spanned_paths
            .iter()
            .map(|p| {
                let path = nu_utils::strip_ansi_string_unlikely(p.item.to_string());
                PathBuf::from(&nu_path::expand_path_with(path, &cwd, p.item.is_expand()))
            })
            .collect();

        let settings = Settings {
            overwrite: overwrite_mode,
            backup: BackupMode::None,
            suffix: OsString::from("~"),
            symbolic,
            logical,
            relative,
            target_dir: None,
            no_target_dir,
            no_dereference,
            verbose,
        };
        if let Err(error) = uu_ln::exec(&paths[..], &settings) {
            if let LnError::SomeLinksFailed = error {
                // We need to set EXIT_CODE just like `Uutils ln`
                stack.set_last_exit_code(1, Span::unknown());
            } else {
                return Err(ShellError::Generic(GenericError::new_internal(
                    format!("{error}"),
                    translate!(&error.to_string()),
                )));
            }
        }

        Ok(PipelineData::empty())
    }
}
