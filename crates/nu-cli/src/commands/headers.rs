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
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let rows: Vec<Value> = input.values.collect().await;

        //the headers are the first row in the table
        let headers: Vec<String> = match &rows[0].value {
            UntaggedValue::Row(d) => {
                Ok(d.entries.iter().map(|(k, v)| {
                    match v.as_string() {
                        Ok(s) => s,
                        Err(_) => { //If a cell that should contain a header name is empty, we name the column Column[index]
                            match d.entries.get_full(k) {
                                Some((index, _, _)) => format!("Column{}", index),
                                None => "unknownColumn".to_string()
                            }
                        }
                    }
                }).collect())
            }
                _ => Err(ShellError::untagged_runtime_error("Couldn't find all headers, was the input a properly formatted, non-empty table?")),
        }?;

        //Each row is a dictionary with the headers as keys
        for r in rows.iter().skip(1) {
            match &r.value {
                UntaggedValue::Row(d) => {
                    let mut i = 0;
                    let mut entries = IndexMap::new();
                    for (_, v) in d.entries.iter() {
                        entries.insert(headers[i].clone(), v.clone());
                        i += 1;
                    }
                    yield Ok(ReturnSuccess::Value(UntaggedValue::Row(Dictionary{entries}).into_value(r.tag.clone())))
                }
                _ => yield Err(ShellError::untagged_runtime_error("Couldn't iterate through rows, was the input a properly formatted table?"))
            }
        }
    };

    Ok(stream.to_output_stream())
}
