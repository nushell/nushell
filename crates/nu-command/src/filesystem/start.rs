use itertools::Itertools;
use nu_engine::{command_prelude::*, env_to_strings};
use nu_protocol::{ShellError, shell_error::generic::GenericError};
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
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
        let path = call
            .req::<Spanned<String>>(engine_state, stack, 0)?
            .map(nu_utils::strip_ansi_string_unlikely);
        let path_no_whitespace = path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        let local_path = match open_target(path_no_whitespace) {
            OpenTarget::Url(url) => {
                open_path(url, engine_state, stack, path.span)?;
                return Ok(PipelineData::empty());
            }
            OpenTarget::Path(path) => path,
        };
        let cwd = engine_state.cwd(Some(stack))?;
        let full_path = nu_path::expand_path_with(local_path, &cwd, true);

        // Check if the path exists or if it's a valid file/directory
        if full_path.exists() {
            open_path(full_path, engine_state, stack, path.span)?;
            return Ok(PipelineData::empty());
        }
        // If neither file nor URL, return an error
        Err(ShellError::Generic(
            GenericError::new(
                format!("Cannot find file or URL: {}", path.item),
                "",
                path.span,
            )
            .with_help("Ensure the path or URL is correct and try again."),
        ))
    }
    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![
            Example {
                description: "Open a text file with the default text editor.",
                example: "start file.txt",
                result: None,
            },
            Example {
                description: "Open an image with the default image viewer.",
                example: "start file.jpg",
                result: None,
            },
            Example {
                description: "Open the current directory with the default file manager.",
                example: "start .",
                result: None,
            },
            Example {
                description: "Open a PDF with the default PDF viewer.",
                example: "start file.pdf",
                result: None,
            },
            Example {
                description: "Open a website with the default browser.",
                example: "start https://www.nushell.sh",
                result: None,
            },
            Example {
                description: "Open an application-registered protocol URL.",
                example: "start obsidian://open?vault=Test",
                result: None,
            },
        ]
    }
}

enum OpenTarget {
    Url(String),
    Path(PathBuf),
}

/// Resolve file URLs to local paths while retaining other URLs for the opener.
fn open_target(input: &str) -> OpenTarget {
    match url::Url::parse(input) {
        Ok(url) if url.scheme().len() == 1 => OpenTarget::Path(PathBuf::from(input)),
        Ok(url) if url.scheme() == "file" => match url.to_file_path() {
            Ok(path) => OpenTarget::Path(path),
            Err(()) => OpenTarget::Url(url.to_string()),
        },
        Ok(url) => OpenTarget::Url(url.to_string()),
        Err(_) => OpenTarget::Path(PathBuf::from(input)),
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

#[cfg(test)]
mod tests {
    use super::{OpenTarget, open_target};
    #[cfg(windows)]
    use std::path::Path;

    #[cfg(windows)]
    #[test]
    fn windows_drive_path_is_not_a_url() {
        assert!(matches!(
            open_target(r"C:\🏠 Commitments\x.txt"),
            OpenTarget::Path(path) if path.as_path() == Path::new(r"C:\🏠 Commitments\x.txt")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn file_urls_open_decoded_local_paths() {
        for input in [
            "file:///C:/umlauts/w%C3%B6rd.odt",
            "file:///C:/umlauts/wörd.odt",
        ] {
            assert!(matches!(
                open_target(input),
                OpenTarget::Path(path) if path.as_path() == Path::new(r"C:\umlauts\wörd.odt")
            ));
        }
    }

    #[test]
    fn web_url_remains_a_url() {
        assert!(matches!(
            open_target("https://example.com/ö"),
            OpenTarget::Url(url) if url == "https://example.com/%C3%B6"
        ));
    }
}
