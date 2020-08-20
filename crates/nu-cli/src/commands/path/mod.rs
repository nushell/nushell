mod basename;
mod command;
mod exists;
mod expand;
mod extension;
mod r#type;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use nu_source::Span;
use std::path::Path;

pub use basename::PathBasename;
pub use command::Path as PathCommand;
pub use exists::PathExists;
pub use expand::PathExpand;
pub use extension::PathExtension;
pub use r#type::PathType;

#[derive(Deserialize)]
struct DefaultArguments {
    rest: Vec<ColumnPath>,
}

fn handle_value<F>(action: &F, v: &Value, span: Span) -> Result<Value, ShellError>
where
    F: Fn(&Path) -> UntaggedValue + Send + 'static,
{
    let v = match &v.value {
        UntaggedValue::Primitive(Primitive::Path(buf)) => action(buf).into_value(v.tag()),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => action(s.as_ref()).into_value(v.tag()),
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
    paths: Vec<ColumnPath>,
    action: &'static F,
    span: Span,
) -> Result<OutputStream, ShellError>
where
    F: Fn(&Path) -> UntaggedValue + Send + Sync + 'static,
{
    Ok(input
        .map(move |v| {
            if paths.is_empty() {
                ReturnSuccess::value(handle_value(&action, &v, span)?)
            } else {
                let mut ret = v;

                for path in &paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| handle_value(&action, &old, span)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}
