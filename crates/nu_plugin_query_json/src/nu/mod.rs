use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::TaggedItem;

use crate::{query_json::begin_json_query, QueryJson};

impl Plugin for QueryJson {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("query json")
            .desc("execute json query on json file (open --raw <file> | query json 'query string')\nsee https://gjson.dev/ for more info.")
            .required("query", SyntaxShape::String, "json query")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let tag = call_info.name_tag;
        let query = call_info.args.nth(0).ok_or_else(|| {
            ShellError::labeled_error("json query not passed", "json query not passed", &tag)
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
            } => match begin_json_query(s, (*self.query).tagged(&self.tag)) {
                Ok(result) => Ok(result.into_iter().map(ReturnSuccess::value).collect()),
                Err(err) => Err(err),
            },
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
