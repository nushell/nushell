use nu_protocol::ast::CellPath;
use nu_protocol::Type;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait Argument {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>>;
}

/// Handle `input` pipeline data as map over table approach.
///
/// In detail, for each elements, if it's value type is valid(indicate through `valid_val_variant`)
/// invoke relative `cmd` with `arg`.
///
/// If `arg` tell us that it's column path is not None, only map over data under these columns.
/// Else it will apply each column inside a table.
///
/// If input type is not valid, it returns `Value::Error` to tell user which type is valid, indicated by `valid_type`
pub fn operate<C, A>(
    cmd: C,
    mut arg: A,
    input: PipelineData,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    valid_val_variant: std::mem::Discriminant<Value>,
    valid_type: Type,
) -> Result<PipelineData, ShellError>
where
    A: Argument + Send + Sync + 'static,
    C: Fn(&Value, &A, Span) -> Value + Send + Sync + 'static + Clone + Copy,
{
    match arg.take_column_paths() {
        None => input.map(
            move |v| {
                let val_discrimiant = std::mem::discriminant(&v);
                if valid_val_variant == val_discrimiant {
                    cmd(&v, &arg, v.span().unwrap_or(span))
                } else {
                    Value::Error {
                        error: ShellError::UnsupportedInput(
                            format!(
                                "Input's type is not supported, support type: {valid_type}, current_type: {}",
                                v.get_type(),
                            ),
                            v.span().unwrap_or(span),
                        ),
                    }
                }
            },
            ctrlc,
        ),
        Some(column_paths) => {
            let arg = Arc::new(arg);

            input.map(
                move |mut v| {
                    for path in &column_paths {
                        let opt = arg.clone();
                        let work_valid_type = valid_type.clone();
                        let r = v.update_cell_path(
                            &path.members,
                            Box::new(move |old| {
                                let val_discrimiant = std::mem::discriminant(old);
                                if valid_val_variant == val_discrimiant {
                                    cmd(old, &opt, old.span().unwrap_or(span))
                                } else {
                                    Value::Error {
                                        error: ShellError::UnsupportedInput(
                                            format!(
                                                "Input's type is not supported. support type: {work_valid_type}, current_type: {}",
                                                old.get_type()
                                            ),
                                            old.span().unwrap_or(span),
                                        ),
                                    }
                                }
                            }),
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
