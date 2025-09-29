#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_protocol::NuGlob;
use uu_mkdir::mkdir;
#[cfg(not(windows))]
use uucore::mode;
use uucore::{localized_help_template, translate};

#[derive(Clone)]
pub struct UMkdir;

const IS_RECURSIVE: bool = true;
const DEFAULT_MODE: u32 = 0o777;

#[cfg(not(windows))]
fn get_mode() -> u32 {
    !mode::get_umask() & DEFAULT_MODE
}

#[cfg(windows)]
fn get_mode() -> u32 {
    DEFAULT_MODE
}

impl Command for UMkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn description(&self) -> &str {
        "Create directories, with intermediary directories if required using uutils/coreutils mkdir."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["directory", "folder", "create", "make_dirs", "coreutils"]
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "rest",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Directory]),
                "The name(s) of the path(s) to create.",
            )
            .switch(
                "verbose",
                "print a message for each created directory.",
                Some('v'),
            )
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
        let _ = localized_help_template("mkdir");

        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;
        let mut directories = call
            .rest::<Spanned<NuGlob>>(engine_state, stack, 0)?
            .into_iter()
            .map(|dir| nu_path::expand_path_with(dir.item.as_ref(), &cwd, dir.item.is_expand()))
            .peekable();

        let is_verbose = call.has_flag(engine_state, stack, "verbose")?;

        if directories.peek().is_none() {
            return Err(ShellError::MissingParameter {
                param_name: "requires directory paths".to_string(),
                span: call.head,
            });
        }

        let config = uu_mkdir::Config {
            recursive: IS_RECURSIVE,
            mode: get_mode(),
            verbose: is_verbose,
            set_selinux_context: false,
            context: None,
        };

        let mut verbose_out = String::new();
        for dir in directories {
            if let Err(error) = mkdir(&dir, &config) {
                return Err(ShellError::GenericError {
                    error: format!("{error}"),
                    msg: translate!(&error.to_string()),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
            if is_verbose {
                verbose_out.push_str(
                    format!(
                        "{} ",
                        &dir.as_path()
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    )
                    .as_str(),
                );
            }
        }

        if is_verbose {
            Ok(PipelineData::value(
                Value::string(verbose_out.trim(), call.head),
                None,
            ))
        } else {
            Ok(PipelineData::empty())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Make a directory named foo",
                example: "mkdir foo",
                result: None,
            },
            Example {
                description: "Make multiple directories and show the paths created",
                example: "mkdir -v foo/bar foo2",
                result: None,
            },
        ]
    }
}
