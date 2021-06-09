use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};

use crate::{selector::begin_selector_query, Selector};

impl Plugin for Selector {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("selector")
            .desc("execute selector query on html/web")
            .required("query", SyntaxShape::String, "selector query")
            .switch("as_html", "return the query output as html", Some('a'))
            .named(
                "attribute",
                SyntaxShape::String,
                "downselect based on the given attribute",
                Some('t'),
            )
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let tag = call_info.name_tag;
        let query = call_info.args.nth(0).ok_or_else(|| {
            ShellError::labeled_error(
                "selector query not passed",
                "selector query not passed",
                &tag,
            )
        })?;

        self.query = query.as_string()?;
        self.tag = tag;
        self.as_html = call_info.args.has("as_html");
        if call_info.args.has("attribute") {
            self.attribute = call_info.args.expect_get("attribute")?.convert_to_string();
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match input {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            } => Ok(begin_selector_query(s, self)
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
