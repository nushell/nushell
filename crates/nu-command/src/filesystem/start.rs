use itertools::Itertools;
use nu_engine::env_to_strings;
use nu_engine::CallExt;
use nu_path::canonicalize_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value;
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
};
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Stdio;

#[derive(Clone)]
pub struct Start;

impl Command for Start {
    fn name(&self) -> &str {
        "start"
    }

    fn usage(&self) -> &str {
        "Open a folder, file or website in the default application or viewer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "folder", "directory", "run", "open"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("start")
            .input_output_types(vec![(Type::Nothing, Type::Any), (Type::String, Type::Any)])
            .optional("path", SyntaxShape::String, "path to open")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: Spanned<String> = match (input, call.opt(engine_state, stack, 0)?) {
            // Positional input
            (
                PipelineData::ExternalStream { stdout: None, .. }
                | PipelineData::Empty
                | PipelineData::Value(Value::Nothing { .. }, _),
                Some(p),
            ) => Ok(p),
            // Pipelined input
            (PipelineData::Value(Value::String { val, .. }, _), None) => Ok(Spanned {
                item: val,
                span: call.head,
            }),
            // Pipelined input from external stream
            (
                PipelineData::ExternalStream {
                    stdout: Some(out), ..
                },
                None,
            ) => out.into_string(),
            // No input is given
            (
                PipelineData::ExternalStream { stdout: None, .. }
                | PipelineData::Empty
                | PipelineData::Value(Value::Nothing { .. }, _),
                None,
            ) => Err(ShellError::MissingParameter {
                param_name: r#"Positional parameter "path" or pipelined data"#.into(),
                span: call.head,
            }),
            // Ambiguous input, unable to determine which is the expected input
            (PipelineData::Value(Value::String { internal_span, .. }, _), Some(p)) => {
                Err(ShellError::IncompatibleParameters {
                    left_message: "Received string from pipelined data".into(),
                    left_span: internal_span,
                    right_message:
                        "...but also a positional parameter, unable to determine which to open"
                            .into(),
                    right_span: p.span,
                })
            }
            (PipelineData::Value(val, _), Some(_)) => Err(ShellError::UnsupportedInput {
                msg:
                    "Got positional parameter and unsupported value from pipeline at the same time"
                        .into(),
                input: "remove pipeline to use positional parameter".into(),
                msg_span: call.head,
                input_span: val.span(),
            }),
            (PipelineData::ListStream(list, _), Some(_)) => Err(ShellError::UnsupportedInput {
                msg:
                    "Got positional parameter and unsupported value from pipeline at the same time"
                        .into(),
                input: "remove pipeline to use positional parameter".into(),
                msg_span: call.head,
                input_span: list.map(|s| s.span()).next().unwrap_or(call.head),
            }),
            (
                PipelineData::ExternalStream {
                    stdout: Some(out), ..
                },
                Some(_),
            ) => Err(ShellError::UnsupportedInput {
                msg:
                    "Got positional parameter and unsupported value from pipeline at the same time"
                        .into(),
                input: "remove pipeline to use positional parameter".into(),
                msg_span: call.head,
                input_span: out.span,
            }),
            // Unsupported input type
            (PipelineData::Value(val, _), None) => Err(ShellError::UnsupportedInput {
                msg: "Only String is allowed here".into(),
                input: "pipelined value from here".into(),
                msg_span: call.head,
                input_span: val.span(),
            }),
            // Unsupported liststream input
            (PipelineData::ListStream(list, _), None) => Err(ShellError::UnsupportedInput {
                msg: "list input is unsupported".into(),
                input: "value originates from here".into(),
                msg_span: call.head,
                input_span: list.map(|s| s.span()).next().unwrap_or(call.head),
            }),
        }?;
        let path = Spanned {
            item: nu_utils::strip_ansi_string_unlikely(path.item),
            span: path.span,
        };
        let path_no_whitespace = &path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        // only check if file exists in current current directory
        let file_path = Path::new(path_no_whitespace);
        if file_path.exists() {
            open_path(path_no_whitespace, engine_state, stack, path.span)?;
        } else if file_path.starts_with("https://") || file_path.starts_with("http://") {
            let url = url::Url::parse(&path.item).map_err(|_| {
                ShellError::GenericError(
                    format!("Cannot parse url: {}", &path.item),
                    "".to_string(),
                    Some(path.span),
                    Some("cannot parse".to_string()),
                    Vec::new(),
                )
            })?;
            open_path(url.as_str(), engine_state, stack, path.span)?;
        } else {
            // try to distinguish between file not found and opening url without prefix
            if let Ok(canon_path) =
                canonicalize_with(path_no_whitespace, std::env::current_dir()?.as_path())
            {
                open_path(canon_path, engine_state, stack, path.span)?;
            } else {
                // open crate does not allow opening URL without prefix
                let path_with_prefix = Path::new("https://").join(&path.item);
                let common_domains = ["com", "net", "org", "edu", "sh"];
                if let Some(url) = path_with_prefix.to_str() {
                    let url = url::Url::parse(url).map_err(|_| {
                        ShellError::GenericError(
                            format!("Cannot parse url: {}", &path.item),
                            "".to_string(),
                            Some(path.span),
                            Some("cannot parse".to_string()),
                            Vec::new(),
                        )
                    })?;
                    if let Some(domain) = url.host() {
                        let domain = domain.to_string();
                        let ext = Path::new(&domain).extension().and_then(|s| s.to_str());
                        if let Some(url_ext) = ext {
                            if common_domains.contains(&url_ext) {
                                open_path(url.as_str(), engine_state, stack, path.span)?;
                            }
                        }
                    }
                    return Err(ShellError::GenericError(
                        format!("Cannot find file or url: {}", &path.item),
                        "".to_string(),
                        Some(path.span),
                        Some("Use prefix https:// to disambiguate URLs from files".to_string()),
                        Vec::new(),
                    ));
                }
            };
        }
        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Open a text file with the default text editor",
                example: "start file.txt",
                result: None,
            },
            Example {
                description: "Open an image with the default image viewer",
                example: "start file.jpg",
                result: None,
            },
            Example {
                description: "Open the current directory with the default file manager",
                example: "start .",
                result: None,
            },
            Example {
                description: "Open a pdf with the default pdf viewer",
                example: "start file.pdf",
                result: None,
            },
            Example {
                description: "Open a website with default browser",
                example: "start https://www.nushell.sh",
                result: None,
            },
        ]
    }
}

fn open_path(
    path: impl AsRef<OsStr>,
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<(), ShellError> {
    try_commands(open::commands(path), engine_state, stack, span)
}

fn try_commands(
    commands: Vec<std::process::Command>,
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<(), ShellError> {
    let env_vars_str = env_to_strings(engine_state, stack)?;
    let cmd_run_result = commands.into_iter().map(|mut cmd| {
        let status = cmd
            .envs(&env_vars_str)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!(
                "\nCommand `{}` failed with {}",
                format_command(&cmd),
                status
            )),
            Err(err) => Err(format!(
                "\nCommand `{}` failed with {}",
                format_command(&cmd),
                err
            )),
        }
    });

    for one_result in cmd_run_result {
        if let Err(err_msg) = one_result {
            return Err(ShellError::ExternalCommand {
                label: "No command found to start with this path".to_string(),
                help: "Try different path or install appropriate command\n".to_string() + &err_msg,
                span,
            });
        }
    }
    Ok(())
}

fn format_command(command: &std::process::Command) -> String {
    let parts_iter = std::iter::repeat(command.get_program())
        .take(1)
        .chain(command.get_args());
    Itertools::intersperse(parts_iter, " ".as_ref())
        .collect::<OsString>()
        .to_string_lossy()
        .into_owned()
}
