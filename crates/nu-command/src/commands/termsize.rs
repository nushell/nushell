use crate::prelude::*;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct TermSize;

#[derive(Deserialize, Clone)]
pub struct TermSizeArgs {
    wide: bool,
    tall: bool,
}

#[async_trait]
impl WholeStreamCommand for TermSize {
    fn name(&self) -> &str {
        "term size"
    }

    fn signature(&self) -> Signature {
        Signature::build("term size")
            .switch("wide", "Report only the width of the terminal", Some('w'))
            .switch("tall", "Report only the height of the terminal", Some('t'))
    }

    fn usage(&self) -> &str {
        "Returns the terminal size as W H"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (TermSizeArgs { wide, tall }, _) = args.process().await?;

        let size = term_size::dimensions();
        match size {
            Some((w, h)) => {
                if wide && !tall {
                    Ok(OutputStream::one(UntaggedValue::int(w).into_value(tag)))
                } else if !wide && tall {
                    Ok(OutputStream::one(UntaggedValue::int(h).into_value(tag)))
                } else {
                    let mut indexmap = IndexMap::with_capacity(2);
                    indexmap.insert("width".to_string(), UntaggedValue::int(w).into_value(&tag));
                    indexmap.insert("height".to_string(), UntaggedValue::int(h).into_value(&tag));
                    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
                    Ok(OutputStream::one(value))
                }
            }
            _ => Ok(OutputStream::one(
                UntaggedValue::string("0 0".to_string()).into_value(tag),
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the width height of the terminal",
                example: "term size",
                result: None,
            },
            Example {
                description: "Return the width of the terminal",
                example: "term size -w",
                result: None,
            },
            Example {
                description: "Return the height (t for tall) of the terminal",
                example: "term size -t",
                result: None,
            },
        ]
    }
}
