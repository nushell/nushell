#[macro_use]
extern crate indexmap;

use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError,
    ShellTypeName, Signature, SpannedItem, SyntaxShape, Tag, Tagged, TaggedItem, Value,
};

struct Embed {
    field: Option<String>,
    values: Vec<Tagged<Value>>,
}
impl Embed {
    fn new() -> Embed {
        Embed {
            field: None,
            values: Vec::new(),
        }
    }

    fn embed(&mut self, value: Tagged<Value>) -> Result<(), ShellError> {
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
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.field = Some(s.clone());
                    self.values = Vec::new();
                }
                value => {
                    return Err(ShellError::type_error(
                        "string",
                        value.type_name().spanned(value.span()),
                    ))
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        self.embed(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        let row = Value::row(indexmap! {
            match &self.field {
                Some(key) => key.clone(),
                None => "root".into(),
            } => Value::table(&self.values).tagged(Tag::unknown()),
        })
        .tagged(Tag::unknown());

        Ok(vec![ReturnSuccess::value(row)])
    }
}

fn main() {
    serve_plugin(&mut Embed::new());
}
