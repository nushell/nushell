use nu_errors::ShellError;

use nu_engine::{CommandArgs, WholeStreamCommand};
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_stream::{OutputStream, ToOutputStream};

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

pub struct Command;

#[derive(Deserialize)]
struct Arguments {
    pub rest: Vec<Value>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Mock echo."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();
        let (Arguments { rest }, input) = args.process().await?;

        let mut base_value = UntaggedValue::string("Yehuda Katz in Ecuador").into_value(name_tag);
        let input: Vec<Value> = input.collect().await;

        if let Some(first) = input.get(0) {
            base_value = first.clone()
        }

        let stream = rest.into_iter().map(move |i| {
            let base_value = base_value.clone();
            match i.as_string() {
                Ok(s) => OutputStream::one(Ok(ReturnSuccess::Value(Value {
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
                                table[0].table_entries().map(Clone::clone).collect();

                            for v in values.iter_mut() {
                                v.tag = base_value.tag();
                            }

                            let subtable =
                                vec![UntaggedValue::Table(values).into_value(base_value.tag())];

                            futures::stream::iter(subtable.into_iter().map(ReturnSuccess::value))
                                .to_output_stream()
                        } else {
                            futures::stream::iter(
                                table
                                    .into_iter()
                                    .map(move |mut v| {
                                        v.tag = base_value.tag();
                                        v
                                    })
                                    .map(ReturnSuccess::value),
                            )
                            .to_output_stream()
                        }
                    }
                    _ => OutputStream::one(Ok(ReturnSuccess::Value(Value {
                        value: i.value.clone(),
                        tag: base_value.tag,
                    }))),
                },
            }
        });

        Ok(futures::stream::iter(stream).flatten().to_output_stream())
    }
}
