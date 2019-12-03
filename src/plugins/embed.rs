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
    are_all_rows: bool,
    values: Vec<Value>,
}
impl Embed {
    fn new() -> Embed {
        Embed {
            field: None,
            are_all_rows: true,
            values: Vec::new(),
        }
    }

    fn embed(&mut self, value: Value) -> Result<(), ShellError> {
        match &value {
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                self.values.push(value);
            }
            _ => {
                self.are_all_rows = false;

                self.values.push(
                    value::row(indexmap! {
                        match &self.field {
                            Some(key) => key.clone(),
                            None => "Column".into()
                        } => value
                    })
                    .into_value(Tag::unknown()),
                );
            }
        }
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
        if self.are_all_rows {
            let row = value::row(indexmap! {
                match &self.field {
                    Some(key) => key.clone(),
                    None => "Column".into(),
                } => value::table(&self.values).into_value(Tag::unknown()),
            })
            .into_untagged_value();

            Ok(vec![ReturnSuccess::value(row)])
        } else {
            Ok(self
                .values
                .iter()
                .map(|row| ReturnSuccess::value(row.clone()))
                .collect::<Vec<_>>())
        }
    }
}

fn main() {
    serve_plugin(&mut Embed::new());
}
