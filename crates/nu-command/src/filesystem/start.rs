use itertools::Itertools;
use nu_engine::{command_prelude::*, env_to_strings};
use nu_path::canonicalize_with;
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Stdio,
};

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
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("path", SyntaxShape::String, "Path to open.")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
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
            let url = url::Url::parse(&path.item).map_err(|_| ShellError::GenericError {
                error: format!("Cannot parse url: {}", &path.item),
                msg: "".to_string(),
                span: Some(path.span),
                help: Some("cannot parse".to_string()),
                inner: vec![],
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
                    let url = url::Url::parse(url).map_err(|_| ShellError::GenericError {
                        error: format!("Cannot parse url: {}", &path.item),
                        msg: "".into(),
                        span: Some(path.span),
                        help: Some("cannot parse".into()),
                        inner: vec![],
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
                    return Err(ShellError::GenericError {
                        error: format!("Cannot find file or url: {}", &path.item),
                        msg: "".into(),
                        span: Some(path.span),
                        help: Some("Use prefix https:// to disambiguate URLs from files".into()),
                        inner: vec![],
                    });
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
        } else if one_result.is_ok() {
            break;
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
