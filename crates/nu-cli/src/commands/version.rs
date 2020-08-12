use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::TaggedListBuilder;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct Version;

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        version(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Display Nu version",
            example: "version",
            result: None,
        }]
    }
}

pub fn version(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.args.span;

    let mut indexmap = IndexMap::with_capacity(4);

    indexmap.insert(
        "version".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    indexmap.insert("features".to_string(), features_enabled(&tag).into_value());

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
}

fn features_enabled(tag: impl Into<Tag>) -> TaggedListBuilder {
    let mut names = TaggedListBuilder::new(tag);

    names.push_untagged(UntaggedValue::string("default"));

    #[cfg(feature = "clipboard-cli")]
    {
        names.push_untagged(UntaggedValue::string("clipboard"));
    }

    #[cfg(feature = "trash-support")]
    {
        names.push_untagged(UntaggedValue::string("trash"));
    }

    #[cfg(feature = "starship-prompt")]
    {
        names.push_untagged(UntaggedValue::string("starship"));
    }

    names
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Version {})
    }
}
