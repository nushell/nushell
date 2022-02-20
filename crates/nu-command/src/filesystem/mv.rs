use std::path::{Path, PathBuf};

// use super::util::get_interactive_confirmation;
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
};

const GLOB_PARAMS: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

#[derive(Clone)]
pub struct Mv;

#[allow(unused_must_use)]
impl Command for Mv {
    fn name(&self) -> &str {
        "mv"
    }

    fn usage(&self) -> &str {
        "Move files or directories."
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
            // .switch("interactive", "ask user to confirm action", Some('i'))
            // .switch("force", "suppress error when no file", Some('f'))
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
        let spanned_destination: Spanned<String> = call.req(engine_state, stack, 1)?;
        // let interactive = call.has_flag("interactive");
        // let force = call.has_flag("force");

        let path = current_dir(engine_state, stack)?;
        let source = path.join(spanned_source.item.as_str());
        let destination = path.join(spanned_destination.item.as_str());

        let mut sources = glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS)
            .map_or_else(|_| Vec::new(), Iterator::collect);

        if sources.is_empty() {
            return Err(ShellError::SpannedLabeledError(
                "Invalid file or pattern".into(),
                "invalid file or pattern".into(),
                spanned_source.span,
            ));
        }

        // We have two possibilities.
        //
        // First, the destination exists.
        //  - If a directory, move everything into that directory, otherwise
        //  - if only a single source, overwrite the file, otherwise
        //  - error.
        //
        // Second, the destination doesn't exist, so we can only rename a single source. Otherwise
        // it's an error.

        if (destination.exists() && !destination.is_dir() && sources.len() > 1)
            || (!destination.exists() && sources.len() > 1)
        {
            return Err(ShellError::SpannedLabeledError(
                "Can only move multiple sources if destination is a directory".into(),
                "destination must be a directory when multiple sources".into(),
                spanned_destination.span,
            ));
        }

        let some_if_source_is_destination = sources
            .iter()
            .find(|f| matches!(f, Ok(f) if destination.starts_with(f)));
        if destination.exists() && destination.is_dir() && sources.len() == 1 {
            if let Some(Ok(filename)) = some_if_source_is_destination {
                return Err(ShellError::SpannedLabeledError(
                    format!(
                        "Not possible to move {:?} to itself",
                        filename.file_name().expect("Invalid file name")
                    ),
                    "cannot move to itself".into(),
                    spanned_destination.span,
                ));
            }
        }

        if let Some(Ok(_filename)) = some_if_source_is_destination {
            sources = sources
                .into_iter()
                .filter(|f| matches!(f, Ok(f) if !destination.starts_with(f)))
                .collect();
        }

        for entry in sources.into_iter().flatten() {
            move_file(
                Spanned {
                    item: entry,
                    span: spanned_source.span,
                },
                Spanned {
                    item: destination.clone(),
                    span: spanned_destination.span,
                },
            )?
        }

        Ok(PipelineData::new(call.head))
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
) -> Result<(), ShellError> {
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
        return Err(ShellError::DirectoryNotFound(to_span));
    }

    let mut to = to;
    if to.is_dir() {
        let from_file_name = match from.file_name() {
            Some(name) => name,
            None => return Err(ShellError::DirectoryNotFound(to_span)),
        };

        to.push(from_file_name);
    }

    move_item(&from, from_span, &to)
}

fn move_item(from: &Path, from_span: Span, to: &Path) -> Result<(), ShellError> {
    // We first try a rename, which is a quick operation. If that doesn't work, we'll try a copy
    // and remove the old file/folder. This is necessary if we're moving across filesystems or devices.
    std::fs::rename(&from, &to).or_else(|_| {
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
            Err(e) => Err(ShellError::SpannedLabeledError(
                format!("Could not move {:?} to {:?}. {:}", from, to, e),
                "could not move".into(),
                from_span,
            )),
        }
    })
}
