mod basename;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
pub mod path_;
mod relative_to;
mod self_;
mod split;
mod r#type;

pub use basename::SubCommand as PathBasename;
pub use dirname::SubCommand as PathDirname;
pub use exists::SubCommand as PathExists;
pub use expand::SubCommand as PathExpand;
pub use join::SubCommand as PathJoin;
pub use parse::SubCommand as PathParse;
pub use path_::PathCommand as Path;
pub use r#type::SubCommand as PathType;
pub use relative_to::SubCommand as PathRelativeTo;
pub use self_::SubCommand as PathSelf;
pub use split::SubCommand as PathSplit;

use nu_protocol::{ShellError, Span, Value};
use std::path::Path as StdPath;

#[cfg(windows)]
const ALLOWED_COLUMNS: [&str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {}

fn operate<F, A>(cmd: &F, args: &A, v: Value, name: Span) -> Value
where
    F: Fn(&StdPath, Span, &A) -> Value + Send + Sync + 'static,
    A: PathSubcommandArguments + Send + Sync + 'static,
{
    let span = v.span();
    match v {
        Value::String { val, .. } => cmd(StdPath::new(&val), span, args),
        _ => handle_invalid_values(v, name),
    }
}

fn handle_invalid_values(rest: Value, name: Span) -> Value {
    Value::error(err_from_value(&rest, name), name)
}

fn err_from_value(rest: &Value, name: Span) -> ShellError {
    match rest {
        Value::Error { error, .. } => *error.clone(),
        _ => {
            if rest.is_nothing() {
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string, record or list".into(),
                    wrong_type: "nothing".into(),
                    dst_span: name,
                    src_span: rest.span(),
                }
            } else {
                ShellError::PipelineMismatch {
                    exp_input_type: "string, row or list".into(),
                    dst_span: name,
                    src_span: rest.span(),
                }
            }
        }
    }
}
