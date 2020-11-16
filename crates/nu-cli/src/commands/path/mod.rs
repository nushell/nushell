mod basename;
mod command;
mod dirname;
mod exists;
mod expand;
mod extension;
mod filestem;
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
pub use r#type::PathType;

#[derive(Deserialize)]
struct DefaultArguments {
    // used by basename, dirname, extension and filestem
    replace: Option<String>,
    // used by filestem
    prefix: Option<String>,
    suffix: Option<String>,
    // used by dirname
    num_levels: Option<u32>,
    // used by all
    paths: Vec<ColumnPath>,
}

fn handle_value<F>(
    action: &F,
    v: &Value,
    span: Span,
    args: Arc<DefaultArguments>,
) -> Result<Value, ShellError>
where
    F: Fn(&Path, Arc<DefaultArguments>) -> UntaggedValue + Send + 'static,
{
    let v = match &v.value {
        UntaggedValue::Primitive(Primitive::Path(buf)) => action(buf, args).into_value(v.tag()),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => {
            action(s.as_ref(), args).into_value(v.tag())
        }
        other => {
            let got = format!("got {}", other.type_name());
            return Err(ShellError::labeled_error_with_secondary(
                "value is not string or path",
                got,
                span,
                "originates from here".to_string(),
                v.tag().span,
            ));
        }
    };
    Ok(v)
}

async fn operate<F>(
    input: crate::InputStream,
    action: &'static F,
    span: Span,
    args: Arc<DefaultArguments>,
) -> Result<OutputStream, ShellError>
where
    F: Fn(&Path, Arc<DefaultArguments>) -> UntaggedValue + Send + Sync + 'static,
{
    Ok(input
        .map(move |v| {
            if args.paths.is_empty() {
                ReturnSuccess::value(handle_value(&action, &v, span, Arc::clone(&args))?)
            } else {
                let mut ret = v;

                for path in &args.paths {
                    let cloned_args = Arc::clone(&args);
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| handle_value(&action, &old, span, cloned_args)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}
