use crate::prelude::*;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{value::StrExt, value::StringExt, Dictionary, Signature, UntaggedValue};

pub mod shadow {
    include!(concat!(env!("OUT_DIR"), "/shadow.rs"));
}

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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        version(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Display Nu version",
            example: "version",
            result: None,
        }]
    }
}

pub fn version(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.args.span;

    let mut indexmap = IndexMap::with_capacity(4);

    indexmap.insert(
        "version".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    let branch: Option<&str> = Some(shadow::BRANCH).filter(|x| !x.is_empty());
    if let Some(branch) = branch {
        indexmap.insert(
            "branch".to_string(),
            branch.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let short_commit: Option<&str> = Some(shadow::SHORT_COMMIT).filter(|x| !x.is_empty());
    if let Some(short_commit) = short_commit {
        indexmap.insert(
            "short_commit".to_string(),
            short_commit.to_pattern_untagged_value().into_value(&tag),
        );
    }
    let commit_hash: Option<&str> = Some(shadow::COMMIT_HASH).filter(|x| !x.is_empty());
    if let Some(commit_hash) = commit_hash {
        indexmap.insert(
            "commit_hash".to_string(),
            commit_hash.to_pattern_untagged_value().into_value(&tag),
        );
    }
    let commit_date: Option<&str> = Some(shadow::COMMIT_DATE).filter(|x| !x.is_empty());
    if let Some(commit_date) = commit_date {
        indexmap.insert(
            "commit_date".to_string(),
            commit_date.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let build_os: Option<&str> = Some(shadow::BUILD_OS).filter(|x| !x.is_empty());
    if let Some(build_os) = build_os {
        indexmap.insert(
            "build_os".to_string(),
            build_os.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let rust_version: Option<&str> = Some(shadow::RUST_VERSION).filter(|x| !x.is_empty());
    if let Some(rust_version) = rust_version {
        indexmap.insert(
            "rust_version".to_string(),
            rust_version.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let rust_channel: Option<&str> = Some(shadow::RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(rust_channel) = rust_channel {
        indexmap.insert(
            "rust_channel".to_string(),
            rust_channel.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let cargo_version: Option<&str> = Some(shadow::CARGO_VERSION).filter(|x| !x.is_empty());
    if let Some(cargo_version) = cargo_version {
        indexmap.insert(
            "cargo_version".to_string(),
            cargo_version.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let pkg_version: Option<&str> = Some(shadow::PKG_VERSION).filter(|x| !x.is_empty());
    if let Some(pkg_version) = pkg_version {
        indexmap.insert(
            "pkg_version".to_string(),
            pkg_version.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let build_time: Option<&str> = Some(shadow::BUILD_TIME).filter(|x| !x.is_empty());
    if let Some(build_time) = build_time {
        indexmap.insert(
            "build_time".to_string(),
            build_time.to_pattern_untagged_value().into_value(&tag),
        );
    }

    let build_rust_channel: Option<&str> =
        Some(shadow::BUILD_RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(build_rust_channel) = build_rust_channel {
        indexmap.insert(
            "build_rust_channel".to_string(),
            build_rust_channel
                .to_pattern_untagged_value()
                .into_value(&tag),
        );
    }

    indexmap.insert(
        "features".to_string(),
        features_enabled().join(", ").to_string_value_create_tag(),
    );

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
}

fn features_enabled() -> Vec<String> {
    let mut names = vec!["default".to_string()];

    #[cfg(feature = "ctrlc")]
    {
        names.push("ctrlc".to_string());
    }

    #[cfg(feature = "dirs")]
    {
        names.push("dirs".to_string());
    }

    #[cfg(feature = "directories")]
    {
        names.push("directories".to_string());
    }

    #[cfg(feature = "ptree")]
    {
        names.push("ptree".to_string());
    }

    // #[cfg(feature = "rich-benchmark")]
    // {
    //     names.push("rich-benchmark".to_string());
    // }

    #[cfg(feature = "rustyline-support")]
    {
        names.push("rustyline".to_string());
    }

    #[cfg(feature = "term")]
    {
        names.push("term".to_string());
    }

    #[cfg(feature = "uuid_crate")]
    {
        names.push("uuid".to_string());
    }

    #[cfg(feature = "which")]
    {
        names.push("which".to_string());
    }

    #[cfg(feature = "ichwh")]
    {
        names.push("ichwh".to_string());
    }

    #[cfg(feature = "zip")]
    {
        names.push("zip".to_string());
    }

    #[cfg(feature = "clipboard-cli")]
    {
        names.push("clipboard-cli".to_string());
    }

    #[cfg(feature = "trash-support")]
    {
        names.push("trash".to_string());
    }

    // #[cfg(feature = "binaryview")]
    // {
    //     names.push("binaryview".to_string());
    // }

    // #[cfg(feature = "start")]
    // {
    //     names.push("start".to_string());
    // }

    // #[cfg(feature = "bson")]
    // {
    //     names.push("bson".to_string());
    // }

    // #[cfg(feature = "sqlite")]
    // {
    //     names.push("sqlite".to_string());
    // }

    // #[cfg(feature = "s3")]
    // {
    //     names.push("s3".to_string());
    // }

    // #[cfg(feature = "chart")]
    // {
    //     names.push("chart".to_string());
    // }

    // #[cfg(feature = "xpath")]
    // {
    //     names.push("xpath".to_string());
    // }

    // #[cfg(feature = "selector")]
    // {
    //     names.push("selector".to_string());
    // }

    // #[cfg(feature = "extra")]
    // {
    //     names.push("extra".to_string());
    // }

    // #[cfg(feature = "preserve_order")]
    // {
    //     names.push("preserve_order".to_string());
    // }

    // #[cfg(feature = "wee_alloc")]
    // {
    //     names.push("wee_alloc".to_string());
    // }

    // #[cfg(feature = "console_error_panic_hook")]
    // {
    //     names.push("console_error_panic_hook".to_string());
    // }

    names.sort();

    names
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Version;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Version {})
    }
}
