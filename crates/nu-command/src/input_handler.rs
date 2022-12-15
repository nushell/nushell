use nu_protocol::ast::CellPath;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait CmdArgument {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>>;
}

/// Arguments with only cell_path.
///
/// If commands is going to use `operate` function, and it only required optional cell_paths
/// Using this to simplify code.
pub struct CellPathOnlyArgs {
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for CellPathOnlyArgs {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl From<Vec<CellPath>> for CellPathOnlyArgs {
    fn from(cell_paths: Vec<CellPath>) -> Self {
        Self {
            cell_paths: (!cell_paths.is_empty()).then_some(cell_paths),
        }
    }
}

/// A simple wrapper for `PipelineData::map` method.
///
/// In detail, for each elements, invoking relative `cmd` with `arg`.
///
/// If `arg` tell us that its cell path is not None, only map over data under these columns.
/// Else it will apply each column inside a table.
///
/// The validation of input element should be handle by `cmd` itself.
pub fn operate<C, A>(
    cmd: C,
    mut arg: A,
    input: PipelineData,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
) -> Result<PipelineData, ShellError>
where
    A: CmdArgument + Send + Sync + 'static,
    C: Fn(&Value, &A, Span) -> Value + Send + Sync + 'static + Clone + Copy,
{
    match arg.take_cell_paths() {
        None => input.map(move |v| cmd(&v, &arg, v.span().unwrap_or(span)), ctrlc),
        Some(column_paths) => {
            let arg = Arc::new(arg);
            input.map(
                move |mut v| {
                    for path in &column_paths {
                        let opt = arg.clone();
                        let r = v.update_cell_path(
                            &path.members,
                            Box::new(move |old| cmd(old, &opt, old.span().unwrap_or(span))),
                        );
                        if let Err(error) = r {
                            return Value::Error { error };
                        }
                    }
                    v
                },
                ctrlc,
            )
        }
    }
}
