use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

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
        version(args, registry)
    }
}

pub fn version(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.call_info.name_tag.clone();

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "version".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
}
