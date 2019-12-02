#[macro_use]
extern crate indexmap;

use nu::{serve_plugin, value, Plugin};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SpannedTypeName, SyntaxShape,
    UntaggedValue, Value,
};
use nu_source::Tag;

struct Embed {
    field: Option<String>,
    values: Vec<Value>,
}
impl Embed {
    fn new() -> Embed {
        Embed {
            field: None,
            values: Vec::new(),
        }
    }

    fn embed(&mut self, value: Value) -> Result<(), ShellError> {
        self.values.push(value);
        Ok(())
    }
}

impl Plugin for Embed {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("embed")
            .desc("Embeds a new field to the table.")
            .optional("field", SyntaxShape::String, "the name of the new column")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.field = Some(s.clone());
                    self.values = Vec::new();
                }
                value => return Err(ShellError::type_error("string", value.spanned_type_name())),
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.embed(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        let row = value::row(indexmap! {
            match &self.field {
                Some(key) => key.clone(),
                None => "root".into(),
            } => value::table(&self.values).into_value(Tag::unknown()),
        })
        .into_untagged_value();

        Ok(vec![ReturnSuccess::value(row)])
    }
}

fn main() {
    serve_plugin(&mut Embed::new());
}
