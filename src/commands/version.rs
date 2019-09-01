use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Dictionary, Value};
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
    let span = args.call_info.name_span;

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "version".to_string(),
        Tagged::from_simple_spanned_item(Value::string(clap::crate_version!()), span),
    );

    let value = Tagged::from_simple_spanned_item(Value::Object(Dictionary::from(indexmap)), span);
    Ok(OutputStream::one(value))
}
