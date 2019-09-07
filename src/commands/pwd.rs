use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::data::{Dictionary, Value};
use crate::parser::registry::Signature;
use crate::prelude::*;
use indexmap::IndexMap;

pub struct PWD;

impl WholeStreamCommand for PWD {
    fn name(&self) -> &str {
        "pwd"
    }

    fn signature(&self) -> Signature {
        Signature::build("pwd")
    }

    fn usage(&self) -> &str {
        "Output the current working directory."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        pwd(args, registry)
    }
}

pub fn pwd(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.call_info.name_span;

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "name".to_string(),
        Tagged::from_simple_spanned_item(Value::string(std::env::current_dir()?.to_string_lossy()), span),
    );

    let value = Tagged::from_simple_spanned_item(Value::Row(Dictionary::from(indexmap)), span);
    Ok(OutputStream::one(value))
}
