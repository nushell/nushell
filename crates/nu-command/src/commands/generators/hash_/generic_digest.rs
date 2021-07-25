use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{ColumnPath, Primitive, UntaggedValue, Value};
use nu_source::Tag;

pub fn run<D>(args: CommandArgs) -> Result<OutputStream, ShellError>
where
    D: digest::Digest,
    digest::Output<D>: core::fmt::LowerHex,
{
    let column_paths: Vec<ColumnPath> = args.rest(0)?;

    Ok(args
        .input
        .map(move |v| {
            if column_paths.is_empty() {
                action::<D>(&v, v.tag())
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action::<D>(old, old.tag())),
                    )?;
                }

                Ok(ret)
            }
        })
        .into_input_stream())
}

pub fn action<D>(input: &Value, tag: Tag) -> Result<Value, ShellError>
where
    D: digest::Digest,
    digest::Output<D>: core::fmt::LowerHex,
{
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let digest_result = D::digest(s.as_bytes());
            Ok(UntaggedValue::string(&format!("{:x}", digest_result)).into_value(tag))
        }
        UntaggedValue::Primitive(Primitive::Binary(bytes)) => {
            let digest_result = D::digest(bytes);
            Ok(UntaggedValue::string(&format!("{:x}", digest_result)).into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not supported for hashing",
                got,
                tag.span,
            ))
        }
    }
}
