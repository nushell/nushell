use nu_errors::ShellError;

use nu_engine::{CommandArgs, WholeStreamCommand};
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_stream::{ActionStream, IntoActionStream};

use serde::Deserialize;

pub struct Command;

#[derive(Deserialize)]
struct Arguments {
    #[allow(unused)]
    pub rest: Vec<Value>,
}

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest("rest", SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Mock echo."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();
        let rest: Vec<Value> = args.rest(0)?;

        let mut base_value = UntaggedValue::string("Yehuda Katz in Ecuador").into_value(name_tag);
        let input: Vec<Value> = args.input.collect();

        if let Some(first) = input.get(0) {
            base_value = first.clone()
        }

        let stream = rest.into_iter().flat_map(move |i| {
            let base_value = base_value.clone();
            match i.as_string() {
                Ok(s) => ActionStream::one(Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag: base_value.tag,
                }))),
                _ => match i {
                    Value {
                        value: UntaggedValue::Table(table),
                        ..
                    } => {
                        if table.len() == 1 && table[0].is_table() {
                            let mut values: Vec<Value> =
                                table[0].table_entries().cloned().collect();

                            for v in &mut values {
                                v.tag = base_value.tag();
                            }

                            let subtable =
                                vec![UntaggedValue::Table(values).into_value(base_value.tag())];

                            (subtable.into_iter().map(ReturnSuccess::value)).into_action_stream()
                        } else {
                            (table
                                .into_iter()
                                .map(move |mut v| {
                                    v.tag = base_value.tag();
                                    v
                                })
                                .map(ReturnSuccess::value))
                            .into_action_stream()
                        }
                    }
                    _ => ActionStream::one(Ok(ReturnSuccess::Value(Value {
                        value: i.value.clone(),
                        tag: base_value.tag,
                    }))),
                },
            }
        });

        Ok(stream.into_action_stream())
    }
}
