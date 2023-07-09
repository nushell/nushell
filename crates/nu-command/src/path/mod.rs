mod basename;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
pub mod path_;
mod relative_to;
mod split;
mod r#type;

use std::path::Path as StdPath;

pub use basename::SubCommand as PathBasename;
pub use dirname::SubCommand as PathDirname;
pub use exists::SubCommand as PathExists;
pub use expand::SubCommand as PathExpand;
pub use join::SubCommand as PathJoin;
pub use parse::SubCommand as PathParse;
pub use path_::PathCommand as Path;
pub use r#type::SubCommand as PathType;
pub use relative_to::SubCommand as PathRelativeTo;
pub use split::SubCommand as PathSplit;

use nu_protocol::{Record, ShellError, Span, Value};

#[cfg(windows)]
const ALLOWED_COLUMNS: [&str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {
    fn get_columns(&self) -> Option<Vec<String>>;
}

fn operate<F, A>(cmd: &F, args: &A, v: Value, name: Span) -> Value
where
    F: Fn(&StdPath, Span, &A) -> Value + Send + Sync + 'static,
    A: PathSubcommandArguments + Send + Sync + 'static,
{
    match v {
        Value::String { val, span } => cmd(StdPath::new(&val), span, args),
        Value::Record { val, span } => {
            let col = if let Some(col) = args.get_columns() {
                col
            } else {
                vec![]
            };
            if col.is_empty() {
                return Value::Error {
                    error: Box::new(ShellError::UnsupportedInput(
                        String::from("when the input is a table, you must specify the columns"),
                        "value originates from here".into(),
                        name,
                        span,
                    )),
                };
            }

            let mut record = Record::new();

            for (k, v) in *val {
                let v = if col.contains(&k) {
                    match v {
                        Value::String { val, span } => cmd(StdPath::new(&val), span, args),
                        _ => return handle_invalid_values(v, name),
                    }
                } else {
                    v
                };

                record.push(k, v);
            }

            Value::record(record, span)
        }
        _ => handle_invalid_values(v, name),
    }
}

fn handle_invalid_values(rest: Value, name: Span) -> Value {
    Value::Error {
        error: Box::new(err_from_value(&rest, name)),
    }
}

fn err_from_value(rest: &Value, name: Span) -> ShellError {
    match rest.span() {
        Ok(span) => {
            if rest.is_nothing() {
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string, record or list".into(),
                    wrong_type: "nothing".into(),
                    dst_span: name,
                    src_span: span,
                }
            } else {
                ShellError::PipelineMismatch {
                    exp_input_type: "string, row or list".into(),
                    dst_span: name,
                    src_span: span,
                }
            }
        }
        Err(error) => error,
    }
}
