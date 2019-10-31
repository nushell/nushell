use crate::commands::WholeStreamCommand;
use crate::data::{Dictionary, Value};
use crate::errors::ShellError;
use crate::parser::registry::Signature;
use crate::prelude::*;
use indexmap::IndexMap;

pub struct Version;

impl WholeStreamCommand for Version {
    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version")
    }

    fn usage(&self) -> &str {
        "Display Nu version"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        date(args, registry)
    }
}

pub fn date(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.call_info.name_tag.clone();

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "version".to_string(),
        Value::string(clap::crate_version!()).tagged(&tag),
    );

    let value = Value::Row(Dictionary::from(indexmap)).tagged(&tag);
    Ok(OutputStream::one(value))
}
