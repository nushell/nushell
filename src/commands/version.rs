use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Dictionary, Value};
use crate::parser::registry::Signature;
use crate::prelude::*;
use indexmap::IndexMap;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Version;

impl WholeStreamCommand for Version {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        date(args, registry)
    }

    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version")
    }
}

pub fn date(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.call_info.name_span;

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "version".to_string(),
        Tagged::from_simple_spanned_item(Value::string(VERSION.to_string()), span),
    );

    let value = Tagged::from_simple_spanned_item(Value::Object(Dictionary::from(indexmap)), span);
    Ok(OutputStream::one(value))
}
