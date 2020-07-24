mod basename;
mod command;
mod expand;
mod extension;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, ShellTypeName, UntaggedValue};
use std::path::Path;

pub use basename::PathBasename;
pub use command::Path as PathCommand;
pub use expand::PathExpand;
pub use extension::PathExtension;

async fn operate<F>(
    args: CommandArgs,
    registry: &CommandRegistry,
    action: F,
) -> Result<OutputStream, ShellError>
where
    F: Fn(&Path) -> UntaggedValue + Send + 'static,
{
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;

    Ok(args
        .input
        .map(move |v| {
            let v = match &v.value {
                UntaggedValue::Primitive(Primitive::Path(buf)) => action(buf).into_value(v.tag()),
                UntaggedValue::Primitive(Primitive::String(s))
                | UntaggedValue::Primitive(Primitive::Line(s)) => {
                    action(s.as_ref()).into_value(v.tag())
                }
                other => {
                    let got = format!("got {}", other.type_name());
                    return Err(ShellError::labeled_error(
                        "value is not string or path",
                        got,
                        v.tag().span,
                    ));
                }
            };

            ReturnSuccess::value(v)
        })
        .to_output_stream())
}
