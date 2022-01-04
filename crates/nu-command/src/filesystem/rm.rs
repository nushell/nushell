#[cfg(unix)]
use std::os::unix::prelude::FileTypeExt;
use std::path::PathBuf;

use super::util::get_interactive_confirmation;

use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct Rm;

// Where self.0 is the unexpanded target's positional index (i.e. call.positional[self.0].span)
struct Target(usize, PathBuf);

struct RmArgs {
    targets: Vec<Target>,
    recursive: bool,
    trash: bool,
    permanent: bool,
    force: bool,
}

impl Command for Rm {
    fn name(&self) -> &str {
        "rm"
    }

    fn usage(&self) -> &str {
        "Remove file(s)."
    }

    fn signature(&self) -> Signature {
        Signature::build("rm")
            .switch(
                "trash",
                "use the platform's recycle bin instead of permanently deleting",
                Some('t'),
            )
            .switch(
                "permanent",
                "don't use recycle bin, delete permanently",
                Some('p'),
            )
            .switch("recursive", "delete subdirectories recursively", Some('r'))
            .switch("force", "suppress error when no file", Some('f'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .rest(
                "rest",
                SyntaxShape::GlobPattern,
                "the file path(s) to remove",
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
        rm(engine_state, stack, call)
    }
}

fn rm(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let trash = call.has_flag("trash");
    let permanent = call.has_flag("permanent");
    let interactive = call.has_flag("interactive");

    if trash && permanent {
        return Err(ShellError::IncompatibleParametersSingle(
            "Can't use \"--trash\" with \"--permanent\"".to_string(),
            call.head,
        ));
    }

    let current_path = current_dir(engine_state, stack)?;
    let mut paths = call
        .rest::<String>(engine_state, stack, 0)?
        .into_iter()
        .map(|path| current_path.join(path))
        .peekable();

    if paths.peek().is_none() {
        return Err(ShellError::FileNotFound(call.positional[0].span));
    }

    // Expand and flatten files
    let resolve_path = |i: usize, path: PathBuf| {
        glob::glob(&path.to_string_lossy()).map_or_else(
            |_| Vec::new(),
            |path_iter| path_iter.flatten().map(|f| Target(i, f)).collect(),
        )
    };

    let mut targets: Vec<Target> = vec![];
    for (i, path) in paths.enumerate() {
        let mut paths: Vec<Target> = resolve_path(i, path);

        if paths.is_empty() {
            return Err(ShellError::FileNotFound(call.positional[i].span));
        }

        targets.append(paths.as_mut());
    }

    let recursive = call.has_flag("recursive");
    let force = call.has_flag("force");

    if interactive && !force {
        let mut remove: Vec<usize> = vec![];
        for (index, file) in targets.iter().enumerate() {
            let prompt: String = format!(
                "Are you sure that you what to delete {}?",
                file.1
                    .file_name()
                    .ok_or_else(|| ShellError::SpannedLabeledError(
                        "File name error".into(),
                        "Unable to get file name".into(),
                        call.head
                    ))?
                    .to_str()
                    .ok_or_else(|| ShellError::SpannedLabeledError(
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
            targets.remove(index);
        }

        if targets.is_empty() {
            return Err(ShellError::NoFileToBeRemoved());
        }
    }

    let args = RmArgs {
        targets,
        recursive,
        trash,
        permanent,
        force,
    };
    let response = rm_helper(call, args);

    // let temp = rm_helper(call, args).flatten();
    // let temp = input.flatten(call.head, move |_| rm_helper(call, args));

    Ok(response
        .into_iter()
        .into_pipeline_data(engine_state.ctrlc.clone()))
    // Ok(Value::Nothing { span })
}

fn rm_helper(call: &Call, args: RmArgs) -> Vec<Value> {
    let (targets, recursive, trash, _permanent, force) = (
        args.targets,
        args.recursive,
        args.trash,
        args.permanent,
        args.force,
    );

    #[cfg(not(feature = "trash-support"))]
    {
        if trash {
            let error = match call.get_flag_expr("trash").ok_or_else(|| {
                ShellError::SpannedLabeledError(
                    "Flag not found".into(),
                    "trash flag not found".into(),
                    call.head,
                )
            }) {
                Ok(expr) => ShellError::FeatureNotEnabled(expr.span),
                Err(err) => err,
            };

            return vec![Value::Error { error }];
        }
    }

    if targets.is_empty() && !force {
        return vec![Value::Error {
            error: ShellError::FileNotFound(call.head),
        }];
    }

    targets
        .into_iter()
        .map(move |target| {
            let (i, f) = (target.0, target.1);

            let is_empty = || match f.read_dir() {
                Ok(mut p) => p.next().is_none(),
                Err(_) => false,
            };

            if let Ok(metadata) = f.symlink_metadata() {
                #[cfg(unix)]
                let is_socket = metadata.file_type().is_socket();
                #[cfg(unix)]
                let is_fifo = metadata.file_type().is_fifo();

                #[cfg(not(unix))]
                let is_socket = false;
                #[cfg(not(unix))]
                let is_fifo = false;

                if metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || recursive
                    || is_socket
                    || is_fifo
                    || is_empty()
                {
                    let result;
                    #[cfg(feature = "trash-support")]
                    {
                        use std::io::Error;
                        result = if trash {
                            trash::delete(&f).map_err(|e: trash::Error| {
                                use std::io::ErrorKind;
                                Error::new(ErrorKind::Other, format!("{:?}", e))
                            })
                        } else if metadata.is_file() {
                            std::fs::remove_file(&f)
                        } else {
                            std::fs::remove_dir_all(&f)
                        };
                    }
                    #[cfg(not(feature = "trash-support"))]
                    {
                        result = if metadata.is_file() || is_socket || is_fifo {
                            std::fs::remove_file(&f)
                        } else {
                            std::fs::remove_dir_all(&f)
                        };
                    }

                    if let Err(e) = result {
                        Value::Error {
                            error: ShellError::RemoveNotPossible(
                                format!("Could not delete because: {:}\nTry '--trash' flag", e),
                                call.head,
                            ),
                        }
                    } else {
                        Value::String {
                            val: format!("deleted {:}", f.to_string_lossy()),
                            span: call.positional[i].span,
                        }
                    }
                } else {
                    Value::Error {
                        error: ShellError::RemoveNotPossible(
                            "Cannot remove. try --recursive".to_string(),
                            call.positional[i].span,
                        ),
                    }
                }
            } else {
                Value::Error {
                    error: ShellError::RemoveNotPossible(
                        "no such file or directory".to_string(),
                        call.positional[i].span,
                    ),
                }
            }
        })
        .collect()
}
