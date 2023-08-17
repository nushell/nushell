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

use nu_protocol::{ShellError, Span, SpannedValue};

#[cfg(windows)]
const ALLOWED_COLUMNS: [&str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {}

fn operate<F, A>(cmd: &F, args: &A, v: SpannedValue, name: Span) -> SpannedValue
where
    F: Fn(&StdPath, Span, &A) -> SpannedValue + Send + Sync + 'static,
    A: PathSubcommandArguments + Send + Sync + 'static,
{
    match v {
        SpannedValue::String { val, span } => cmd(StdPath::new(&val), span, args),
        _ => handle_invalid_values(v, name),
    }
}

fn handle_invalid_values(rest: SpannedValue, name: Span) -> SpannedValue {
    SpannedValue::Error {
        error: Box::new(err_from_value(&rest, name)),
        span: name,
    }
}

fn err_from_value(rest: &SpannedValue, name: Span) -> ShellError {
    match rest {
        SpannedValue::Error { error, span } => *error.clone(),
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
