use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::ffi::OsString;

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
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .switch("recursive", "copy directories recursively", Some('r'))
            .switch("verbose", "explicitly state what is being done", Some('v'))
            .switch("force", "if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). currently not implemented for windows", Some('f'))
            .switch("interactive", "ask before overwriting files", Some('i'))
            .switch("progress", "display a progress bar", Some('g'))
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
                description: "Copy many file recursively into a new folder showing a progress bar",
                example: "cp -r -g big_folder new_folder",
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
        // [OPTIONS] SOURCE DEST
        // -r, --recursive     copy directories recursively [short aliases: R]
        // -v, --verbose       explicitly state what is being done (also adds --debug)
        // -f, --force         if an existing destination file cannot be opened, remove it and try again
        //                     (this option is ignored when the -n option is also used). Currently not
        //                     implemented for Windows.
        // -i, --interactive   ask before overwriting files
        // -g, --progress      Display a progress bar.
        let src: Spanned<String> = call.req(engine_state, stack, 0)?;
        let src = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(src.item),
                span: src.span,
            }
        };
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let progress = call.has_flag("progress");

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

        let mut args: Vec<OsString> = vec!["cp".into()]; // seed it with the cp command
        if recursive {
            // working
            args.push("-r".into());
        }
        if verbose {
            // working
            args.push("-v".into());
            args.push("--debug".into());
        }
        if interactive {
            // working
            args.push("-i".into());
        }
        if progress {
            // working (you won't see it unless there are a lot of files)
            args.push("-g".into());
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
        uu_cp::uumain(args.into_iter());
        Ok(PipelineData::empty())
    }
}
