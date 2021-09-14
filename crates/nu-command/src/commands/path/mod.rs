mod basename;
mod command;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
mod relative_to;
mod split;
mod r#type;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Dictionary, MaybeOwned, Primitive, ShellTypeName, UntaggedValue, Value,
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
pub use relative_to::PathRelativeTo;
pub use split::PathSplit;

#[cfg(windows)]
const ALLOWED_COLUMNS: [&str; 4] = ["prefix", "parent", "stem", "extension"];
#[cfg(not(windows))]
const ALLOWED_COLUMNS: [&str; 3] = ["parent", "stem", "extension"];

trait PathSubcommandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath>;
}

fn encode_path(
    entries: &Dictionary,
    orig_span: Span,
    new_span: Span,
) -> Result<PathBuf, ShellError> {
    if entries.length() == 0 {
        return Err(ShellError::labeled_error_with_secondary(
            "Empty table cannot be encoded as a path",
            "got empty table",
            new_span,
            "originates from here",
            orig_span,
        ));
    }

    for col in entries.keys() {
        if !ALLOWED_COLUMNS.contains(&col.as_str()) {
            let msg = format!(
                "Column '{}' is not valid for a structured path. Allowed columns are: {}",
                col,
                ALLOWED_COLUMNS.join(", ")
            );
            return Err(ShellError::labeled_error_with_secondary(
                "Expected structured path table",
                msg,
                new_span,
                "originates from here",
                orig_span,
            ));
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
        let p = val.as_string()?;
        if !p.is_empty() {
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

fn join_path(parts: &[Value], new_span: &Span) -> Result<PathBuf, ShellError> {
    parts
        .iter()
        .map(|part| match &part.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(Path::new(s)),
            UntaggedValue::Primitive(Primitive::FilePath(pb)) => Ok(pb.as_path()),
            _ => {
                let got = format!("got {}", part.type_name());
                Err(ShellError::labeled_error_with_secondary(
                    "Cannot join values that are not paths or strings.",
                    got,
                    new_span,
                    "originates from here",
                    part.tag.span,
                ))
            }
        })
        .collect()
}

fn handle_value<F, T>(action: &F, v: &Value, span: Span, args: Arc<T>) -> Result<Value, ShellError>
where
    T: PathSubcommandArguments,
    F: Fn(&Path, Tag, &T) -> Value,
{
    match &v.value {
        UntaggedValue::Primitive(Primitive::FilePath(buf)) => Ok(action(buf, v.tag(), &args)),
        UntaggedValue::Primitive(Primitive::String(s)) => Ok(action(s.as_ref(), v.tag(), &args)),
        UntaggedValue::Row(entries) => {
            // implicit path join makes all subcommands understand the structured path
            let path_buf = encode_path(entries, v.tag().span, span)?;
            Ok(action(&path_buf, v.tag(), &args))
        }
        UntaggedValue::Table(parts) => {
            // implicit path join makes all subcommands understand path split into parts
            let path_buf = join_path(parts, &span)?;
            Ok(action(&path_buf, v.tag(), &args))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error_with_secondary(
                "Value is a not string, path, row, or table",
                got,
                span,
                "originates from here",
                v.tag().span,
            ))
        }
    }
}

fn operate_column_paths<F, T>(
    input: crate::InputStream,
    action: &'static F,
    span: Span,
    args: Arc<T>,
) -> OutputStream
where
    T: PathSubcommandArguments + Send + Sync + 'static,
    F: Fn(&Path, Tag, &T) -> Value + Send + Sync + 'static,
{
    input
        .map(move |v| {
            let mut ret = v;

            for path in args.get_column_paths() {
                let cloned_args = Arc::clone(&args);
                ret = match ret.swap_data_by_column_path(
                    path,
                    Box::new(move |old| handle_value(&action, old, span, cloned_args)),
                ) {
                    Ok(v) => v,
                    Err(e) => Value::error(e),
                };
            }

            ret
        })
        .into_output_stream()
}

fn operate<F, T>(
    input: crate::InputStream,
    action: &'static F,
    span: Span,
    args: Arc<T>,
) -> OutputStream
where
    T: PathSubcommandArguments + Send + Sync + 'static,
    F: Fn(&Path, Tag, &T) -> Value + Send + Sync + 'static,
{
    if args.get_column_paths().is_empty() {
        input
            .map(
                move |v| match handle_value(&action, &v, span, Arc::clone(&args)) {
                    Ok(v) => v,
                    Err(e) => Value::error(e),
                },
            )
            .into_output_stream()
    } else {
        operate_column_paths(input, action, span, args)
    }
}

fn column_paths_from_args(args: &CommandArgs) -> Result<Vec<ColumnPath>, ShellError> {
    let column_paths: Option<Vec<Value>> = args.get_flag("columns")?;
    let has_columns = column_paths.is_some();
    let column_paths = match column_paths {
        Some(cols) => {
            let mut c = Vec::new();
            for col in cols {
                let colpath = ColumnPath::build(&col.convert_to_string().spanned_unknown());
                if !colpath.is_empty() {
                    c.push(colpath)
                }
            }
            c
        }
        None => Vec::new(),
    };

    if has_columns && column_paths.is_empty() {
        let colval: Option<Value> = args.get_flag("columns")?;
        let colspan = match colval {
            Some(v) => v.tag.span,
            None => Span::unknown(),
        };
        return Err(ShellError::labeled_error(
            "Requires a list of columns",
            "must be a list of columns",
            colspan,
        ));
    }

    Ok(column_paths)
}
