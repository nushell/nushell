mod length;
mod starts_with;
use nu_protocol::ast::CellPath;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use length::BytesLen;
pub use starts_with::BytesStartsWith;

trait BytesArgument {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>>;
}

fn operate<C, A>(
    cmd: C,
    mut arg: A,
    input: PipelineData,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
) -> Result<PipelineData, ShellError>
where
    A: BytesArgument + Send + Sync + 'static,
    C: Fn(&Value, &A, Span) -> Value + Send + Sync + 'static + Clone + Copy,
{
    match arg.take_column_paths() {
        None => input.map(move |v| cmd(&v, &arg, span), ctrlc),
        Some(column_paths) => {
            let arg = Arc::new(arg);
            input.map(
                move |mut v| {
                    for path in &column_paths {
                        let opt = arg.clone();
                        let r = v.update_cell_path(
                            &path.members,
                            Box::new(move |old| cmd(old, &opt, span)),
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
