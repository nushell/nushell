use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::ffi::OsString;

#[derive(Clone)]
pub struct Umv;

impl Command for Umv {
    fn name(&self) -> &str {
        "mv"
    }

    fn usage(&self) -> &str {
        "Move files using uutils/coreutils mv."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["move", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("mv")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .switch(
                "strip-trailing-slashes",
                "remove any trailing slashes from each SOURCE argument",
                Some('s'),
            )
            .switch("verbose", "explain what is being done", Some('v'))
            .switch("force", "do not prompt before overwriting", Some('f'))
            .switch("interactive", "prompt before override", Some('i'))
            .switch("progress", "display a progress bar", Some('g'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Move myfile to dir_b",
                example: "mv myfile dir_b",
                result: None,
            },
            Example {
                description: "Force move dir_a to dir_b",
                example: "mv -f dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move dir_a to dir_b, and print the feedbacks",
                example: "mv -v dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move many file recursively into a new folder showing a progress bar",
                example: "mv -p big_folder new_folder",
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
        // Get the App
        // let mut app = uu_cp::uu_app();
        // app.print_help()?;

        // MVP args
        // Usage: mv [OPTION]... SOURCE DEST
        // Rename SOURCE to DEST, or move SOURCE(s) to DIRECTORY.

        // Mandatory arguments to long options are mandatory for short options too.
        //   -f, --force                  do not prompt before overwriting
        //   -i, --interactive            prompt before overwrite
        // If you specify more than one of -i, -f, -n, only the final one takes effect.
        //   -s, --strip-trailing-slashes  remove any trailing slashes from each SOURCE argument
        //   -p, --progress               show progress bar
        //   -n, --no-clobber             do not overwrite an existing file
        //   -v, --verbose                explain what is being done

        let src: Spanned<String> = call.req(engine_state, stack, 0)?;
        let src = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(src.item),
                span: src.span,
            }
        };
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let force = call.has_flag("force");
        let interactive = call.has_flag("interactive");
        let strip_trailing_slashes = call.has_flag("strip-trailing-slashes");
        let progress = call.has_flag("progress");
        let no_clobber = call.has_flag("no-clobber");
        let verbose = call.has_flag("verbose");

        let current_dir_path = current_dir(engine_state, stack)?;
        let source = current_dir_path.join(src.item.as_str());
        let destination = current_dir_path.join(dst.item.as_str());
        // let ctrlc = engine_state.ctrlc.clone();
        // let span = call.head;

        // POC
        // Create uucore::Args somehow from nushell args
        // let s1 = "cp".to_string();
        // let s2 = "-h".to_string();
        // let args = vec![OsString::from(s1), OsString::from(s2)];

        let mut args: Vec<OsString> = vec!["mv".into()]; // seed it with the cp command
        if strip_trailing_slashes {
            // working
            args.push("--strip-trailing-slashes".into());
        }
        if verbose {
            // working
            args.push("-v".into());
        }
        if force {
            // working
            args.push("-f".into());
        }
        if interactive {
            // working
            args.push("-i".into());
        }
        if progress {
            // working (you won't see it unless there are a lot of files)
            args.push("-p".into());
        }
        if no_clobber {
            args.push("-n".into());
        }
        let src_input = match source.to_str() {
            Some(s) => s,
            None => {
                return Err(ShellError::GenericError(
                    "No source input provided".into(),
                    "No source input provided".into(),
                    Some(src.span),
                    None,
                    Vec::new(),
                ))
            }
        };
        let dest_input = match destination.to_str() {
            Some(d) => d,
            None => {
                return Err(ShellError::GenericError(
                    "No destination input provided".into(),
                    "No destination input provided".into(),
                    Some(dst.span),
                    None,
                    Vec::new(),
                ))
            }
        };

        args.push(src_input.into());
        args.push(dest_input.into());

        // Pass uucore::Args to app.uumain
        uu_mv::uumain(args.into_iter());
        Ok(PipelineData::empty())
    }
}
