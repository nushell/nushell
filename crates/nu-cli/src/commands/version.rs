use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use last_git_commit::LastGitCommit;
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

    let mut indexmap = IndexMap::with_capacity(3);

    indexmap.insert(
        "version".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    if let Ok(lgc) = LastGitCommit::new().build() {
        indexmap.insert(
            "short_commit_hash".to_string(),
            UntaggedValue::string(format!("{}", lgc.id().short())).into_value(&tag),
        );
        indexmap.insert(
            "long_commit_hash".to_string(),
            UntaggedValue::string(format!("{}", lgc.id().long())).into_value(&tag),
        );
    }

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
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
