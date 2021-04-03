mod basename;
mod command;
mod dirname;
mod exists;
mod expand;
mod extension;
mod filestem;
mod join;
mod parse;
mod r#type;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use nu_source::Span;
use std::path::Path;
use std::sync::Arc;

pub use basename::PathBasename;
pub use command::Path as PathCommand;
pub use dirname::PathDirname;
pub use exists::PathExists;
pub use expand::PathExpand;
pub use extension::PathExtension;
pub use filestem::PathFilestem;
pub use join::PathJoin;
pub use parse::PathParse;
pub use r#type::PathType;

trait PathSubcommandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath>;
}

fn handle_value<F, T>(action: &F, v: &Value, span: Span, args: Arc<T>) -> Result<Value, ShellError>
where
    T: PathSubcommandArguments + Send + 'static,
    F: Fn(&Path, Tag, &T) -> Result<Value, ShellError> + Send + 'static,
{
    match &v.value {
        UntaggedValue::Primitive(Primitive::FilePath(buf)) => {
            // action(buf, &args).into_value(v.tag())
            action(buf, v.tag(), &args)
        }
        UntaggedValue::Primitive(Primitive::String(s)) => {
            // action(s.as_ref(), &args).into_value(v.tag())
            action(s.as_ref(), v.tag(), &args)
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error_with_secondary(
                "value is not string or path",
                got,
                span,
                "originates from here".to_string(),
                v.tag().span,
            ))
        }
    }
}

fn operate<F, T>(
    input: crate::InputStream,
    action: &'static F,
    span: Span,
    args: Arc<T>,
) -> ActionStream
where
    T: PathSubcommandArguments + Send + Sync + 'static,
    F: Fn(&Path, Tag, &T) -> Result<Value, ShellError> + Send + Sync + 'static,
{
    input
        .map(move |v| {
            if args.get_column_paths().is_empty() {
                ReturnSuccess::value(handle_value(&action, &v, span, Arc::clone(&args))?)
            } else {
                let mut ret = v;

                for path in args.get_column_paths() {
                    let cloned_args = Arc::clone(&args);
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| handle_value(&action, &old, span, cloned_args)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_action_stream()
}
