mod basename;
mod command;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
mod split;
mod r#type;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Dictionary, MaybeOwned, Primitive, ReturnSuccess, ShellTypeName, UntaggedValue,
    Value,
};
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
pub use split::PathSplit;

// columns of a structured path
#[cfg(windows)]
const ALLOWED_COLUMNS: [&str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath>;
}

fn join_path(entries: &Dictionary) -> Result<PathBuf, ShellError> {
    if entries.length() == 0 {
        return Err(ShellError::untagged_runtime_error(
            "Empty rows are not allowed",
        ));
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

    #[cfg(windows)]
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

    if !basename.is_empty() {
        result.push(basename);
    }

    Ok(result)
}

fn handle_value<F, T>(action: &F, v: &Value, span: Span, args: Arc<T>) -> Result<Value, ShellError>
where
    T: PathSubcommandArguments + Send + 'static,
    F: Fn(&Path, Tag, &T) -> Result<Value, ShellError> + Send + 'static,
{
    match &v.value {
        UntaggedValue::Primitive(Primitive::FilePath(buf)) => action(buf, v.tag(), &args),
        UntaggedValue::Primitive(Primitive::String(s)) => action(s.as_ref(), v.tag(), &args),
        UntaggedValue::Row(entries) => {
            // implicit join makes all subcommands understand the structured path
            let path_buf = join_path(entries)?;
            action(&path_buf, v.tag(), &args)
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error_with_secondary(
                "value is not string, path or a table with relevant columns",
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
