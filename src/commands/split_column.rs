use crate::errors::ShellError;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use log::trace;

pub fn split_column(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let positional: Vec<_> = args.positional_iter().cloned().collect();
    let span = args.call_info.name_span;

    if positional.len() == 0 {
        return Err(ShellError::labeled_error(
            "Split-column needs more information",
            "needs parameter (eg split-column \",\")",
            args.call_info.name_span,
        ));
    }

    let input = args.input;

    Ok(input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(ref s)) => {
                let splitter = positional[0].as_string().unwrap().replace("\\n", "\n");
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
                        dict.insert(
                            v.as_string().unwrap(),
                            Value::Primitive(Primitive::String(k.into())),
                        );
                    }
                    ReturnSuccess::value(dict.into_tagged_value())
                } else {
                    let mut dict = TaggedDictBuilder::new(v.tag());
                    for k in positional.iter().skip(1) {
                        dict.insert(k.as_string().unwrap().trim(), Primitive::String("".into()));
                    }
                    ReturnSuccess::value(dict.into_tagged_value())
                }
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected a string from pipeline",
                "requires string input",
                span,
                "value originates from here",
                v.span(),
            )),
        })
        .to_output_stream())
}
