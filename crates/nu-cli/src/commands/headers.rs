use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::Dictionary;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Headers;
#[derive(Deserialize)]
pub struct HeadersArgs {}

impl WholeStreamCommand for Headers {
    fn name(&self) -> &str {
        "headers"
    }
    fn signature(&self) -> Signature {
        Signature::build("headers")
    }
    fn usage(&self) -> &str {
        "Use the first row of the table as headers"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, headers)?.run()
    }
}

pub fn headers(
    HeadersArgs {}: HeadersArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let rows: Vec<Value> = input.values.collect().await;

        //the headers are the first row in the table
        let headers: Vec<String> = match &rows[0].value {
            UntaggedValue::Row(d) => {
                d.entries.iter().map(|(_, v)| {
                    match v.as_string() {
                        Ok(s) => s,
                        Err(_) => String::from("empty-header") //If a cell that should contain a header name is empty, we need to fill it with something.
                    }
                }).collect()
            }
            _ => panic!("Could not find headers")
        };

        //Each row is a dictionary with the headers as keys
        let newrows: Vec<Value> = rows.iter().skip(1).map(|r| {
            match &r.value {
                UntaggedValue::Row(d) => {

                    let mut i = 0;
                    let mut entries = IndexMap::new();
                    for (_, v) in d.entries.iter() {
                        entries.insert(headers[i].clone(), v.clone());
                        i += 1;
                    }

                    UntaggedValue::Row(Dictionary{entries}).into_value(r.tag.clone())
                }
                _ => panic!("Row value was not an UntaggedValue::Row")
            }
        }).collect();

        yield ReturnSuccess::value(UntaggedValue::table(&newrows).into_value(name))
    };

    Ok(stream.to_output_stream())
}
