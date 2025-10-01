use itertools::Itertools;
use nu_engine::{command_prelude::*, env_to_strings};
use nu_protocol::ShellError;
use std::{
    ffi::{OsStr, OsString},
    process::Stdio,
};

#[derive(Clone)]
pub struct Start;

impl Command for Start {
    fn name(&self) -> &str {
        "start"
    }

    fn description(&self) -> &str {
        "Open a folder, file, or website in the default application or viewer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "folder", "directory", "run", "open"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("start")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("path", SyntaxShape::String, "Path or URL to open.")
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
        let path_no_whitespace = path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        // Attempt to parse the input as a URL
        if let Ok(url) = url::Url::parse(path_no_whitespace) {
            open_path(url.as_str(), engine_state, stack, path.span)?;
            return Ok(PipelineData::empty());
        }
        // If it's not a URL, treat it as a file path
        let cwd = engine_state.cwd(Some(stack))?;
        let full_path = nu_path::expand_path_with(path_no_whitespace, &cwd, true);

        // Check if the path exists or if it's a valid file/directory
        if full_path.exists() {
            open_path(full_path, engine_state, stack, path.span)?;
            return Ok(PipelineData::empty());
        }
        // If neither file nor URL, return an error
        Err(ShellError::GenericError {
            error: format!("Cannot find file or URL: {}", &path.item),
            msg: "".into(),
            span: Some(path.span),
            help: Some("Ensure the path or URL is correct and try again.".into()),
            inner: vec![],
        })
    }
    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
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
                description: "Open a PDF with the default PDF viewer",
                example: "start file.pdf",
                result: None,
            },
            Example {
                description: "Open a website with the default browser",
                example: "start https://www.nushell.sh",
                result: None,
            },
            Example {
                description: "Open an application-registered protocol URL",
                example: "start obsidian://open?vault=Test",
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
    let mut last_err = None;

    for mut cmd in commands {
        let status = cmd
            .envs(&env_vars_str)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                last_err = Some(format!(
                    "Command `{}` failed with exit code: {}",
                    format_command(&cmd),
                    status.code().unwrap_or(-1)
                ));
            }
            Err(err) => {
                last_err = Some(format!(
                    "Command `{}` failed with error: {}",
                    format_command(&cmd),
                    err
                ));
            }
        }
    }

    Err(ShellError::ExternalCommand {
        label: "Failed to start the specified path or URL".to_string(),
        help: format!(
            "Try a different path or install the appropriate application.\n{}",
            last_err.unwrap_or_default()
        ),
        span,
    })
}

fn format_command(command: &std::process::Command) -> String {
    let parts_iter = std::iter::once(command.get_program()).chain(command.get_args());
    Itertools::intersperse(parts_iter, OsStr::new(" "))
        .collect::<OsString>()
        .to_string_lossy()
        .into_owned()
}
