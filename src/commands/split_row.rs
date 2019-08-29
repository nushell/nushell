use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

#[derive(Deserialize)]
struct SplitRowArgs {
    separator: Tagged<String>,
}

pub struct SplitRow;

impl WholeStreamCommand for SplitRow {
    fn name(&self) -> &str {
        "split-row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-row")
            .required("separator", SyntaxType::Any)
    }

    fn usage(&self) -> &str {
        "Split row contents over multiple rows via the separator."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, split_row)?.run()
    }
}

fn split_row(
    SplitRowArgs { separator }: SplitRowArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(ref s)) => {
                let splitter = separator.item.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        Value::Primitive(Primitive::String(s.into())).tagged(v.tag()),
                    ));
                }
                result
            }
            _ => {
                let mut result = VecDeque::new();
                result.push_back(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name,
                    "value originates from here",
                    v.span(),
                )));
                result
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
