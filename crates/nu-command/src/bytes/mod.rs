mod add;
mod at;
mod build_;
mod bytes_;
mod collect;
mod ends_with;
mod index_of;
mod length;
mod remove;
mod replace;
mod reverse;
mod starts_with;

use nu_protocol::ast::CellPath;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use add::BytesAdd;
pub use at::BytesAt;
pub use build_::BytesBuild;
pub use bytes_::Bytes;
pub use collect::BytesCollect;
pub use ends_with::BytesEndsWith;
pub use index_of::BytesIndexOf;
pub use length::BytesLen;
pub use remove::BytesRemove;
pub use replace::BytesReplace;
pub use reverse::BytesReverse;
pub use starts_with::BytesStartsWith;

trait BytesArgument {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>>;
}

/// map input pipeline data, for each elements, if it's Binary, invoke relative `cmd` with `arg`.
fn operate<C, A>(
    cmd: C,
    mut arg: A,
    input: PipelineData,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
) -> Result<PipelineData, ShellError>
where
    A: BytesArgument + Send + Sync + 'static,
    C: Fn(&[u8], &A, Span) -> Value + Send + Sync + 'static + Clone + Copy,
{
    match arg.take_column_paths() {
        None => input.map(
            move |v| match v {
                Value::Binary {
                    val,
                    span: val_span,
                } => cmd(&val, &arg, val_span),
                other => Value::Error {
                    error: ShellError::UnsupportedInput(
                        format!(
                            "Input's type is {}. This command only works with bytes.",
                            other.get_type()
                        ),
                        span,
                    ),
                },
            },
            ctrlc,
        ),
        Some(column_paths) => {
            let arg = Arc::new(arg);
            input.map(
                move |mut v| {
                    for path in &column_paths {
                        let opt = arg.clone();
                        let r = v.update_cell_path(
                            &path.members,
                            Box::new(move |old| {
                                match old {
                                    Value::Binary {val, span: val_span} => cmd(val, &opt, *val_span),
                                    other => Value::Error {
                                    error: ShellError::UnsupportedInput(
                                        format!(
                                            "Input's type is {}. This command only works with bytes.",
                                            other.get_type()
                                        ),
                                        span,
                                 ),
                            }}}),
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
