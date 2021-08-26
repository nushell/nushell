use std::marker::PhantomData;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, SyntaxShape, UntaggedValue, Value};
use nu_protocol::{ShellTypeName, Signature};
use nu_source::Tag;

pub trait HashDigest: digest::Digest {
    fn name() -> &'static str;
    fn examples() -> Vec<Example>;
}

pub struct SubCommand<D: HashDigest> {
    name_string: String,
    usage_string: String,
    phantom: PhantomData<D>,
}

impl<D: HashDigest> Default for SubCommand<D> {
    fn default() -> Self {
        Self {
            name_string: format!("hash {}", D::name()),
            usage_string: format!("{} encode a value", D::name()),
            phantom: PhantomData,
        }
    }
}

impl<D> WholeStreamCommand for SubCommand<D>
where
    D: HashDigest + Send + Sync,
    digest::Output<D>: core::fmt::LowerHex,
{
    fn name(&self) -> &str {
        &self.name_string
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).rest(
            "rest",
            SyntaxShape::ColumnPath,
            format!("optionally {} encode data by column paths", D::name()),
        )
    }

    fn usage(&self) -> &str {
        &self.usage_string
    }

    fn examples(&self) -> Vec<Example> {
        D::examples()
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
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
}

pub fn action<D>(input: &Value, tag: Tag) -> Result<Value, ShellError>
where
    D: HashDigest,
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
                format!("value is not supported for hashing as {}", D::name()),
                got,
                tag.span,
            ))
        }
    }
}
