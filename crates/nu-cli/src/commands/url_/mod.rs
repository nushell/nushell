mod command;
mod host;
mod path;
mod query;
mod scheme;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use url::Url;

pub use command::Url as UrlCommand;
pub use host::UrlHost;
pub use path::UrlPath;
pub use query::UrlQuery;
pub use scheme::UrlScheme;

#[derive(Deserialize)]
struct DefaultArguments {
    rest: Vec<ColumnPath>,
}

fn handle_value<F>(action: &F, v: &Value) -> Result<Value, ShellError>
where
    F: Fn(&Url) -> &str + Send + 'static,
{
    let a = |url| UntaggedValue::string(action(url));
    let v = match &v.value {
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => match Url::parse(s) {
            Ok(url) => a(&url).into_value(v.tag()),
            Err(_) => UntaggedValue::string("").into_value(v.tag()),
        },
        other => {
            let got = format!("got {}", other.type_name());
            return Err(ShellError::labeled_error(
                "value is not a string",
                got,
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
) -> Result<OutputStream, ShellError>
where
    F: Fn(&Url) -> &str + Send + Sync + 'static,
{
    Ok(input
        .map(move |v| {
            if paths.is_empty() {
                ReturnSuccess::value(handle_value(&action, &v)?)
            } else {
                let mut ret = v;

                for path in &paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| handle_value(&action, &old)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}
