use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::TaggedItem;

use crate::{xpath::string_to_value, Xpath};

impl Plugin for Xpath {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("xpath")
            .desc("execute xpath query on xml")
            .required("query", SyntaxShape::String, "xpath query")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let tag = call_info.name_tag;

        let query = call_info.args.nth(0).ok_or_else(|| {
            ShellError::labeled_error("xpath query not passed", "xpath query not passed", &tag)
        })?;

        self.query = query.as_string()?;
        self.tag = tag;

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match input {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            } => Ok(string_to_value(s, (*self.query).tagged(&self.tag))?
                .into_iter()
                .map(ReturnSuccess::value)
                .collect()),
            Value { tag, .. } => Err(ShellError::labeled_error_with_secondary(
                "Expected text from pipeline",
                "requires text input",
                &self.tag,
                "value originates from here",
                tag,
            )),
        }
    }
}
