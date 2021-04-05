mod basename;
mod command;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
mod r#type;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, MaybeOwned, Primitive, ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use nu_source::Span;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use basename::PathBasename;
pub use command::Path as PathCommand;
pub use dirname::PathDirname;
pub use exists::PathExists;
pub use expand::PathExpand;
pub use join::PathJoin;
pub use parse::PathParse;
pub use r#type::PathType;

// columns of a structured path
#[cfg(windows)]
const ALLOWED_COLUMNS: [&'static str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&'static str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath>;
}

fn into_path(v: &Value, span: Span) -> Result<PathBuf, ShellError> {
    match &v.value {
        UntaggedValue::Primitive(Primitive::FilePath(path_buf)) => Ok(path_buf.into()),
        UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.into()),
        UntaggedValue::Row(entries) => {
            if entries.length() == 0 {
                return Err(ShellError::untagged_runtime_error("Empty rows are not allowed"));
            }

            //TODO: check out `get` error for wrong column:
            //   There isn't a column named 'name'
            //   Perhaps you meant 'parent'? Columns available: parent, stem, extension
            for col in entries.keys() {
                if !ALLOWED_COLUMNS.contains(&col.as_str()) {
                    let msg = format!("Column '{}' is not allowed", col);
                    return Err(ShellError::untagged_runtime_error(msg));
                }
            }

            // At this point, the row is known to have >0 columns, all of them allowed
            let mut result = PathBuf::new();

            // #[cfg(windows)]
            if let MaybeOwned::Borrowed(val) = entries.get_data("prefix") {
                let s = val.as_string()?;
                if !s.is_empty() {
                    result.push(&s);
                }
            };

            if let MaybeOwned::Borrowed(val) = entries.get_data("parent") {
                let p = val.as_filepath()?;
                if !p.as_os_str().is_empty() {
                    result.push(p);
                }
            };

            let mut basename = String::new();

            if let MaybeOwned::Borrowed(val) = entries.get_data("stem") {
                let s = val.as_string()?;
                if !s.is_empty() {
                    basename.push_str(&s);
                }
            };

            if let MaybeOwned::Borrowed(val) = entries.get_data("extension") {
                let s = val.as_string()?;
                if !s.is_empty() {
                    basename.push('.');
                    basename.push_str(&s);
                }
            };

            result.push(basename);

            Ok(result)
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

fn handle_value<F, T>(action: &F, v: &Value, span: Span, args: Arc<T>) -> Result<Value, ShellError>
where
    T: PathSubcommandArguments + Send + 'static,
    F: Fn(&Path, Tag, &T) -> Result<Value, ShellError> + Send + 'static,
{
    let pb = into_path(v, span)?;
    action(&pb, v.tag(), &args)
    // match &v.value {
    //     UntaggedValue::Primitive(Primitive::FilePath(buf)) => {
    //         // action(buf, &args).into_value(v.tag())
    //         action(buf, v.tag(), &args)
    //     }
    //     UntaggedValue::Primitive(Primitive::String(s)) => {
    //         // action(s.as_ref(), &args).into_value(v.tag())
    //         action(s.as_ref(), v.tag(), &args)
    //     }
    //     other => {
    //         let got = format!("got {}", other.type_name());
    //         Err(ShellError::labeled_error_with_secondary(
    //             "value is not string or path",
    //             got,
    //             span,
    //             "originates from here".to_string(),
    //             v.tag().span,
    //         ))
    //     }
    // }
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
