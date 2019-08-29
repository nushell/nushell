use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use log::trace;

#[derive(Deserialize)]
struct SplitColumnArgs {
    separator: Tagged<String>,
    rest: Vec<Tagged<String>>,
    #[serde(rename(deserialize = "collapse-empty"))]
    collapse_empty: bool,
}

pub struct SplitColumn;

impl WholeStreamCommand for SplitColumn {
    fn name(&self) -> &str {
        "split-column"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-column")
            .required("separator", SyntaxType::Any)
            .switch("collapse-empty")
            .rest(SyntaxType::Member)
    }

    fn usage(&self) -> &str {
        "Split row contents across multiple columns via the separator."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, split_column)?.run()
    }
}

fn split_column(
    SplitColumnArgs { separator, rest, collapse_empty}: SplitColumnArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(ref s)) => {
                let splitter = separator.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);

                let split_result: Vec<_> = if collapse_empty {
                    s.split(&splitter).filter(|s| !s.is_empty()).collect()
                } else {
                    s.split(&splitter).collect()
                };

                trace!("split result = {:?}", split_result);

                let positional: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

                // If they didn't provide column names, make up our own
                if positional.len() == 0 {
                    let mut gen_columns = vec![];
                    for i in 0..split_result.len() {
                        gen_columns.push(format!("Column{}", i + 1));
                    }

                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for (&k, v) in split_result.iter().zip(gen_columns.iter()) {
                        dict.insert(v.clone(), Primitive::String(k.into()));
                    }

                    ReturnSuccess::value(dict.into_tagged_value())
                } else if split_result.len() == positional.len() {
                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for (&k, v) in split_result.iter().zip(positional.iter()) {
                        dict.insert(v, Value::Primitive(Primitive::String(k.into())));
                    }
                    ReturnSuccess::value(dict.into_tagged_value())
                } else {
                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for (&k, v) in split_result.iter().zip(positional.iter()) {
                        dict.insert(v, Value::Primitive(Primitive::String(k.into())));
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
