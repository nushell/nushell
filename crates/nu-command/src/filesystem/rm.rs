use std::env::current_dir;
#[cfg(unix)]
use std::os::unix::prelude::FileTypeExt;
use std::path::PathBuf;

use super::interactive_helper::get_confirmation;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, SyntaxShape, Value, ValueStream};

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
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<Value, ShellError> {
        rm(context, call)
    }
}

fn rm(context: &EvaluationContext, call: &Call) -> Result<Value, ShellError> {
    let trash = call.has_flag("trash");
    let permanent = call.has_flag("permanent");
    let interactive = call.has_flag("interactive");

    if trash && permanent {
        return Err(ShellError::IncompatibleParametersSingle(
            "Can't use \"--trash\" with \"--permanent\"".to_string(),
            call.head,
        ));

        //     let trash_span = call.get_flag_expr("trash").unwrap().span;
        //     let perm_span = call.get_flag_expr("permanent").unwrap().span;

        //     let left_message = "cannot use".to_string();
        //     let right_message = "with".to_string();
        //     let (left_span, right_span) = match trash_span.start < perm_span.start {
        //         true => (trash_span, perm_span),
        //         false => (perm_span, trash_span),
        //     };

        //     return Err(ShellError::IncompatibleParameters {
        //         left_message,
        //         left_span,
        //         right_message,
        //         right_span,
        //     });
    }

    let current_path = current_dir()?;
    let mut paths = call
        .rest::<String>(context, 0)?
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
        let mut remove_index: Vec<usize> = vec![];
        for (index, file) in targets.iter().enumerate() {
            let prompt: String = format!(
                "Are you sure that you what to delete {}?",
                file.1.file_name().unwrap().to_str().unwrap()
            );

            let input = get_confirmation(prompt)?;

            if !input {
                remove_index.push(index);
            }
        }

        for index in remove_index {
            targets.remove(index);
        }

        if targets.len() == 0 {
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

    Ok(Value::Stream {
        stream: ValueStream::from_stream(response.into_iter()),
        span: call.head,
    })

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
            return vec![Value::Error {
                error: ShellError::FeatureNotEnabled(call.get_flag_expr("trash").unwrap().span),
            }];
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
