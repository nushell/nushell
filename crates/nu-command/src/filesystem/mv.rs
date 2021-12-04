use std::env::current_dir;
use std::path::{Path, PathBuf};

use super::util::get_interactive_confirmation;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

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
            .switch("interactive", "ask user to confirm action", Some('i'))
            .switch("force", "suppress error when no file", Some('f'))
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
        let destination: String = call.req(engine_state, stack, 1)?;
        let interactive = call.has_flag("interactive");
        let force = call.has_flag("force");

        let path: PathBuf = current_dir()?;
        let source = path.join(spanned_source.item.as_str());
        let destination = path.join(destination.as_str());

        let mut sources =
            glob::glob(&source.to_string_lossy()).map_or_else(|_| Vec::new(), Iterator::collect);

        if sources.is_empty() {
            return Err(ShellError::FileNotFound(spanned_source.span));
        }

        if interactive && !force {
            let mut remove: Vec<usize> = vec![];
            for (index, file) in sources.iter().enumerate() {
                let prompt = format!(
                    "Are you shure that you want to move {} to {}?",
                    file.as_ref()
                        .map_err(|err| ShellError::LabeledError(
                            "Reference error".into(),
                            err.to_string(),
                            call.head
                        ))?
                        .file_name()
                        .ok_or_else(|| ShellError::LabeledError(
                            "File name error".into(),
                            "Unable to get file name".into(),
                            call.head
                        ))?
                        .to_str()
                        .ok_or_else(|| ShellError::LabeledError(
                            "Unable to get str error".into(),
                            "Unable to convert to str file name".into(),
                            call.head
                        ))?,
                    destination
                        .file_name()
                        .ok_or_else(|| ShellError::LabeledError(
                            "File name error".into(),
                            "Unable to get file name".into(),
                            call.head
                        ))?
                        .to_str()
                        .ok_or_else(|| ShellError::LabeledError(
                            "Unable to get str error".into(),
                            "Unable to convert to str file name".into(),
                            call.head
                        ))?,
                );

                let input = get_interactive_confirmation(prompt)?;

                if !input {
                    remove.push(index);
                }
            }

            remove.reverse();

            for index in remove {
                sources.remove(index);
            }

            if sources.is_empty() {
                return Err(ShellError::NoFileToBeMoved());
            }
        }

        if (destination.exists() && !destination.is_dir() && sources.len() > 1)
            || (!destination.exists() && sources.len() > 1)
        {
            return Err(ShellError::MoveNotPossible {
                source_message: "Can't move many files".to_string(),
                source_span: call.positional[0].span,
                destination_message: "into single file".to_string(),
                destination_span: call.positional[1].span,
            });
        }

        let some_if_source_is_destination = sources
            .iter()
            .find(|f| matches!(f, Ok(f) if destination.starts_with(f)));
        if destination.exists() && destination.is_dir() && sources.len() == 1 {
            if let Some(Ok(_filename)) = some_if_source_is_destination {
                return Err(ShellError::MoveNotPossible {
                    source_message: "Can't move directory".to_string(),
                    source_span: call.positional[0].span,
                    destination_message: "into itself".to_string(),
                    destination_span: call.positional[1].span,
                });
            }
        }

        if let Some(Ok(_filename)) = some_if_source_is_destination {
            sources = sources
                .into_iter()
                .filter(|f| matches!(f, Ok(f) if !destination.starts_with(f)))
                .collect();
        }

        for entry in sources.into_iter().flatten() {
            move_file(call, &entry, &destination)?
        }

        Ok(PipelineData::new(call.head))
    }
}

fn move_file(call: &Call, from: &Path, to: &Path) -> Result<(), ShellError> {
    if to.exists() && from.is_dir() && to.is_file() {
        return Err(ShellError::MoveNotPossible {
            source_message: "Can't move a directory".to_string(),
            source_span: call.positional[0].span,
            destination_message: "to a file".to_string(),
            destination_span: call.positional[1].span,
        });
    }

    let destination_dir_exists = if to.is_dir() {
        true
    } else {
        to.parent().map(Path::exists).unwrap_or(true)
    };

    if !destination_dir_exists {
        return Err(ShellError::DirectoryNotFound(call.positional[1].span));
    }

    let mut to = to.to_path_buf();
    if to.is_dir() {
        let from_file_name = match from.file_name() {
            Some(name) => name,
            None => return Err(ShellError::DirectoryNotFound(call.positional[1].span)),
        };

        to.push(from_file_name);
    }

    move_item(call, from, &to)
}

fn move_item(call: &Call, from: &Path, to: &Path) -> Result<(), ShellError> {
    // We first try a rename, which is a quick operation. If that doesn't work, we'll try a copy
    // and remove the old file/folder. This is necessary if we're moving across filesystems or devices.
    std::fs::rename(&from, &to).map_err(|_| ShellError::MoveNotPossible {
        source_message: "failed to move".to_string(),
        source_span: call.positional[0].span,
        destination_message: "into".to_string(),
        destination_span: call.positional[1].span,
    })
}
