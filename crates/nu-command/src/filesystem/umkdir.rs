use nu_engine::command_prelude::*;
use nu_protocol::{NuGlob, shell_error::generic::GenericError};
use uu_mkdir::mkdir;
use uucore::{localized_help_template, translate};

#[derive(Clone)]
pub struct UMkdir;

const IS_RECURSIVE: bool = true;
const DEFAULT_MODE: u32 = 0o777;

#[cfg(target_family = "unix")]
fn get_mode() -> u32 {
    !nu_system::get_umask() & DEFAULT_MODE
}

#[cfg(not(target_family = "unix"))]
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
        vec![
            "directory",
            "folder",
            "create",
            "make_dirs",
            "coreutils",
            "md",
        ]
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (
                    Type::Nothing,
                    Type::Table(
                        [
                            ("path".to_string(), Type::String),
                            ("created".to_string(), Type::Bool),
                            (
                                "error".to_string(),
                                Type::OneOf([Type::Nothing, Type::String].into()),
                            ),
                        ]
                        .into(),
                    ),
                ),
            ])
            .rest(
                "rest",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Directory]),
                "The name(s) of the path(s) to create.",
            )
            .switch(
                "verbose",
                "Print a message for each created directory.",
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

        let cwd = engine_state.cwd(Some(stack))?.into_std_path_buf();
        let mut directories = call
            .rest::<Spanned<NuGlob>>(engine_state, stack, 0)?
            .into_iter()
            .map(|dir| {
                (
                    nu_path::expand_path_with(dir.item.as_ref(), &cwd, dir.item.is_expand()),
                    dir.span,
                )
            })
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
            verbose: false,
            set_security_context: false,
            context: None,
        };

        let mut verbose_out = Vec::new();
        let mut err = None;
        for (dir, dir_span) in directories {
            if let Err(error) = mkdir(&dir, &config) {
                let shell_error = ShellError::Generic(GenericError::new(
                    format!("{error}"),
                    translate!(&error.to_string()),
                    dir_span,
                ));

                if is_verbose {
                    verbose_out.push(
                        record! {
                            "path" => Value::string(dir.display().to_string(), call.head),
                            "created" => Value::bool(false, call.head),
                            "error" => Value::string(format!("{error}"), call.head),
                        }
                        .into_value(call.head),
                    )
                } else {
                    err = Some(shell_error);
                }
            } else if is_verbose {
                verbose_out.push(
                    record! {
                        "path" => Value::string(dir.display().to_string(), call.head),
                        "created" => Value::bool(true, call.head),
                        "error" => Value::nothing(call.head),
                    }
                    .into_value(call.head),
                );
            }
        }

        if is_verbose {
            Ok(PipelineData::value(
                Value::list(verbose_out, call.head),
                None,
            ))
        } else if let Some(err) = err {
            Err(err)
        } else {
            Ok(PipelineData::empty())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Make a directory named foo.",
                example: "mkdir foo",
                result: None,
            },
            Example {
                description: "Make multiple directories and show the paths created.",
                example: "mkdir -v foo/bar foo2",
                result: Some(Value::test_list(vec![
                    Value::record(
                        record! {
                            "path" => Value::string("foo/bar".to_string(), Span::test_data()),
                            "created" => Value::bool(true, Span::test_data()),
                            "error" => Value::nothing(Span::test_data()),
                        },
                        Span::test_data(),
                    ),
                    Value::record(
                        record! {
                            "path" => Value::string("foo2".to_string(), Span::test_data()),
                            "created" => Value::bool(true, Span::test_data()),
                            "error" => Value::nothing(Span::test_data()),
                        },
                        Span::test_data(),
                    ),
                ])),
            },
        ]
    }
}
