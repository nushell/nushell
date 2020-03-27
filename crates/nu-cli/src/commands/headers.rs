use crate::commands::WholeStreamCommand;
use indexmap::IndexMap;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};
use std::fs::File;
use std::io::Write;

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
        args.process(registry, count)?.run()
    }
}

//Rows is an array of dictionaries. Each dictionary maps header to content for that row.
//Take the first row and extract all content and save them as headers.
//Take the rest of the rows and replace the old column names with the new headers.

pub fn count(
    HeadersArgs {}: HeadersArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {


    let stream = async_stream! {
        let rows: Vec<Value> = input.values.collect().await;

        let mut entries = IndexMap::new();
        let headers: Value = rows[0].clone();
        match &headers.value {
            UntaggedValue::Row(d) => {
                entries = d.entries.clone();
                ()
            }
            _ => ()
        }
        let mut heads = vec![];
        for (k, v) in entries.iter() {
            heads.push(v.as_string().unwrap());
        }

        let mut file =  File::create("headout").unwrap();
        write!(file, "args: {:#?}", heads);

        yield ReturnSuccess::value(UntaggedValue::int(rows.len()).into_value(name))
    };

    Ok(stream.to_output_stream())
}
