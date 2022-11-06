use std::path::{Path, PathBuf};

use super::util::try_interaction;
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};

const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

#[derive(Clone)]
pub struct Mv;

impl Command for Mv {
    fn name(&self) -> &str {
        "mv"
    }

    fn usage(&self) -> &str {
        "Move files or directories."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["move"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("mv")
            .required(
                "source",
                SyntaxShape::GlobPattern,
                "the location to move files/directories from",
            )
            .required(
                "destination",
                SyntaxShape::Filepath,
                "the location to move files/directories to",
            )
            .switch(
                "verbose",
                "make mv to be verbose, showing files been moved.",
                Some('v'),
            )
            .switch("force", "overwrite the destination.", Some('f'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        // TODO: handle invalid directory or insufficient permissions when moving
        let spanned_source: Spanned<String> = call.req(engine_state, stack, 0)?;
        let spanned_source = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(spanned_source.item),
                span: spanned_source.span,
            }
        };
        let spanned_destination: Spanned<String> = call.req(engine_state, stack, 1)?;
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let force = call.has_flag("force");

        let ctrlc = engine_state.ctrlc.clone();

        let path = current_dir(engine_state, stack)?;
        let source = path.join(spanned_source.item.as_str());
        let destination = path.join(spanned_destination.item.as_str());

        let mut sources = nu_glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS)
            .map_or_else(|_| Vec::new(), Iterator::collect);

        if sources.is_empty() {
            return Err(ShellError::GenericError(
                "Invalid file or pattern".into(),
                "invalid file or pattern".into(),
                Some(spanned_source.span),
                None,
                Vec::new(),
            ));
        }

        // We have two possibilities.
        //
        // First, the destination exists.
        //  - If a directory, move everything into that directory, otherwise
        //  - if only a single source, and --force (or -f) is provided overwrite the file,
        //  - otherwise error.
        //
        // Second, the destination doesn't exist, so we can only rename a single source. Otherwise
        // it's an error.

        if destination.exists() && !force && !destination.is_dir() && !source.is_dir() {
            return Err(ShellError::GenericError(
                "Destination file already exists".into(),
                "you can use -f, --force to force overwriting the destination".into(),
                Some(spanned_destination.span),
                None,
                Vec::new(),
            ));
        }

        if (destination.exists() && !destination.is_dir() && sources.len() > 1)
            || (!destination.exists() && sources.len() > 1)
        {
            return Err(ShellError::GenericError(
                "Can only move multiple sources if destination is a directory".into(),
                "destination must be a directory when multiple sources".into(),
                Some(spanned_destination.span),
                None,
                Vec::new(),
            ));
        }

        if source.is_dir() && destination.is_dir() {
            if let Some(name) = source.file_name() {
                let dst = destination.join(name);
                if dst.is_dir() {
                    return Err(ShellError::GenericError(
                        format!("Can't move {:?} to {:?}", source, dst),
                        "Directory not empty".into(),
                        Some(spanned_destination.span),
                        None,
                        Vec::new(),
                    ));
                }
            }
        }

        let some_if_source_is_destination = sources
            .iter()
            .find(|f| matches!(f, Ok(f) if destination.starts_with(f)));
        if destination.exists() && destination.is_dir() && sources.len() == 1 {
            if let Some(Ok(filename)) = some_if_source_is_destination {
                return Err(ShellError::GenericError(
                    format!(
                        "Not possible to move {:?} to itself",
                        filename.file_name().unwrap_or(filename.as_os_str())
                    ),
                    "cannot move to itself".into(),
                    Some(spanned_destination.span),
                    None,
                    Vec::new(),
                ));
            }
        }

        if let Some(Ok(_filename)) = some_if_source_is_destination {
            sources.retain(|f| matches!(f, Ok(f) if !destination.starts_with(f)));
        }

        let span = call.head;
        Ok(sources
            .into_iter()
            .flatten()
            .filter_map(move |entry| {
                let result = move_file(
                    Spanned {
                        item: entry.clone(),
                        span: spanned_source.span,
                    },
                    Spanned {
                        item: destination.clone(),
                        span: spanned_destination.span,
                    },
                    interactive,
                );
                if let Err(error) = result {
                    Some(Value::Error { error })
                } else if verbose {
                    let val = match result {
                        Ok(true) => format!(
                            "moved {:} to {:}",
                            entry.to_string_lossy(),
                            destination.to_string_lossy()
                        ),
                        _ => format!(
                            "{:} not moved to {:}",
                            entry.to_string_lossy(),
                            destination.to_string_lossy()
                        ),
                    };
                    Some(Value::String { val, span })
                } else {
                    None
                }
            })
            .into_pipeline_data(ctrlc))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a file",
                example: "mv before.txt after.txt",
                result: None,
            },
            Example {
                description: "Move a file into a directory",
                example: "mv test.txt my/subdirectory",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "mv *.txt my/subdirectory",
                result: None,
            },
        ]
    }
}

fn move_file(
    spanned_from: Spanned<PathBuf>,
    spanned_to: Spanned<PathBuf>,
    interactive: bool,
) -> Result<bool, ShellError> {
    let Spanned {
        item: from,
        span: from_span,
    } = spanned_from;
    let Spanned {
        item: to,
        span: to_span,
    } = spanned_to;

    if to.exists() && from.is_dir() && to.is_file() {
        return Err(ShellError::MoveNotPossible {
            source_message: "Can't move a directory".to_string(),
            source_span: spanned_from.span,
            destination_message: "to a file".to_string(),
            destination_span: spanned_to.span,
        });
    }

    let destination_dir_exists = if to.is_dir() {
        true
    } else {
        to.parent().map(Path::exists).unwrap_or(true)
    };

    if !destination_dir_exists {
        return Err(ShellError::DirectoryNotFound(to_span, None));
    }

    // This can happen when changing case on a case-insensitive filesystem (ex: changing foo to Foo on Windows)
    // When it does, we want to do a plain rename instead of moving `from` into `to`
    let from_to_are_same_file = same_file::is_same_file(&from, &to).unwrap_or(false);

    let mut to = to;
    if !from_to_are_same_file && to.is_dir() {
        let from_file_name = match from.file_name() {
            Some(name) => name,
            None => return Err(ShellError::DirectoryNotFound(to_span, None)),
        };

        to.push(from_file_name);
    }

    if interactive && to.exists() {
        let (interaction, confirmed) =
            try_interaction(interactive, "mv: overwrite", &to.to_string_lossy());
        if let Err(e) = interaction {
            return Err(ShellError::GenericError(
                format!("Error during interaction: {:}", e),
                "could not move".into(),
                None,
                None,
                Vec::new(),
            ));
        } else if !confirmed {
            return Ok(false);
        }
    }

    match move_item(&from, from_span, &to) {
        Ok(()) => Ok(true),
        Err(e) => Err(e),
    }
}

fn move_item(from: &Path, from_span: Span, to: &Path) -> Result<(), ShellError> {
    // We first try a rename, which is a quick operation. If that doesn't work, we'll try a copy
    // and remove the old file/folder. This is necessary if we're moving across filesystems or devices.
    std::fs::rename(from, to).or_else(|_| {
        match if from.is_file() {
            let mut options = fs_extra::file::CopyOptions::new();
            options.overwrite = true;
            fs_extra::file::move_file(from, to, &options)
        } else {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;
            fs_extra::dir::move_dir(from, to, &options)
        } {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_kind = match e.kind {
                    fs_extra::error::ErrorKind::Io(io) => {
                        format!("I/O error: {}", io)
                    }
                    fs_extra::error::ErrorKind::StripPrefix(sp) => {
                        format!("Strip prefix error: {}", sp)
                    }
                    fs_extra::error::ErrorKind::OsString(os) => {
                        format!("OsString error: {:?}", os.to_str())
                    }
                    _ => e.to_string(),
                };
                Err(ShellError::GenericError(
                    format!(
                        "Could not move {:?} to {:?}. Error Kind: {}",
                        from, to, error_kind
                    ),
                    "could not move".into(),
                    Some(from_span),
                    None,
                    Vec::new(),
                ))
            }
        }
    })
}
