use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use log::trace;

#[derive(Deserialize)]
struct SplitColumnArgs {
    rest: Vec<Tagged<String>>,
}

pub struct SplitColumn;

impl WholeStreamCommand for SplitColumn {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, split_column)?.run()
    }

    fn name(&self) -> &str {
        "split-column"
    }

    fn signature(&self) -> Signature {
        // TODO: Improve error. Old error had extra info:
        //
        //   needs parameter (e.g. split-column ",")
        Signature::build("split-column").rest()
    }
}

fn split_column(
    SplitColumnArgs { rest: positional }: SplitColumnArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(ref s)) => {
                let positional: Vec<_> = positional.iter().map(|f| f.item.clone()).collect();

                // TODO: Require at least 1 positional argument.
                let splitter = positional[0].replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                // If they didn't provide column names, make up our own
                if (positional.len() - 1) == 0 {
                    let mut gen_columns = vec![];
                    for i in 0..split_result.len() {
                        gen_columns.push(format!("Column{}", i + 1));
                    }

                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for (&k, v) in split_result.iter().zip(gen_columns.iter()) {
                        dict.insert(v.clone(), Primitive::String(k.into()));
                    }

                    ReturnSuccess::value(dict.into_tagged_value())
                } else if split_result.len() == (positional.len() - 1) {
                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for (&k, v) in split_result.iter().zip(positional.iter().skip(1)) {
                        dict.insert(v, Value::Primitive(Primitive::String(k.into())));
                    }
                    ReturnSuccess::value(dict.into_tagged_value())
                } else {
                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for k in positional.iter().skip(1) {
                        dict.insert(k.trim(), Primitive::String("".into()));
                    }
                    ReturnSuccess::value(dict.into_tagged_value())
                }
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected a string from pipeline",
                "requires string input",
                name,
                "value originates from here",
                v.span(),
            )),
        })
        .to_output_stream())
}
