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

use nu_protocol::{ShellError, Span, Value};

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
        Value::Record { cols, vals, span } => {
            let col = if let Some(col) = args.get_columns() {
                col
            } else {
                vec![]
            };
            if col.is_empty() {
                return Value::Error {
                    error: ShellError::UnsupportedInput(
                        String::from("when the input is a table, you must specify the columns"),
                        name,
                    ),
                };
            }

            let mut output_cols = vec![];
            let mut output_vals = vec![];

            for (k, v) in cols.iter().zip(vals) {
                output_cols.push(k.clone());
                if col.contains(k) {
                    let new_val = match v {
                        Value::String { val, span } => cmd(StdPath::new(&val), span, args),
                        _ => return handle_invalid_values(v, name),
                    };
                    output_vals.push(new_val);
                } else {
                    output_vals.push(v);
                }
            }

            Value::Record {
                cols: output_cols,
                vals: output_vals,
                span,
            }
        }
        _ => handle_invalid_values(v, name),
    }
}

fn handle_invalid_values(rest: Value, name: Span) -> Value {
    Value::Error {
        error: err_from_value(&rest, name),
    }
}

fn err_from_value(rest: &Value, name: Span) -> ShellError {
    match rest.span() {
        Ok(span) => {
            if rest.is_nothing() {
                ShellError::UnsupportedInput(
                    "Input type is nothing, expected: string, row or list".into(),
                    name,
                )
            } else {
                ShellError::PipelineMismatch("string, row or list".into(), name, span)
            }
        }
        Err(error) => error,
    }
}
