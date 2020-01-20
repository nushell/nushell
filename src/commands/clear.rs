use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct Clear;

impl WholeStreamCommand for Clear {
    fn name(&self) -> &str {
        "clear"
    }
    fn signature(&self) -> Signature {
        Signature::build("clear")
    }
    fn usage(&self) -> &str {
        "clears the terminal"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        clear(args, registry)
    }
}
pub fn clear(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.call_info.name_tag.clone();

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "clear".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
}
