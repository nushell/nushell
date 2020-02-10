use crate::Average;
use nu_errors::{CoerceInto, ShellError};
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value,
};
use nu_source::TaggedItem;

impl Plugin for Average {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("average")
            .desc("Compute the average of a column of numerical values.")
            .filter())
    }

    fn begin_filter(&mut self, _: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.average(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        match self.total {
            None => Ok(vec![]),
            Some(ref inner) => match &inner.value {
                UntaggedValue::Primitive(Primitive::Int(i)) => {
                    let total: u64 = i
                        .tagged(inner.tag.clone())
                        .coerce_into("converting for average")?;
                    let avg = total as f64 / self.count as f64;
                    let primitive_value: UntaggedValue = Primitive::from(avg).into();
                    let value = primitive_value.into_value(inner.tag.clone());
                    Ok(vec![ReturnSuccess::value(value)])
                }
                UntaggedValue::Primitive(Primitive::Bytes(bytes)) => {
                    let avg = *bytes as f64 / self.count as f64;
                    let primitive_value: UntaggedValue = UntaggedValue::bytes(avg as u64);
                    let tagged_value = primitive_value.into_value(inner.tag.clone());
                    Ok(vec![ReturnSuccess::value(tagged_value)])
                }
                _ => Ok(vec![]),
            },
        }
    }
}
