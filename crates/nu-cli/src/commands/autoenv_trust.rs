use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use std::io::Write;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::AnchorLocation;
use std::fs::OpenOptions;

pub struct AutoenvTrust;

#[async_trait]
impl WholeStreamCommand for AutoenvTrust {
    fn name(&self) -> &str {
        "autoenv trust"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv trust")
    }

    fn usage(&self) -> &str {
        "Trust a .nu-env file in the current directory"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("autoenv.txt")
            .unwrap();

        write!(&mut file, "I'm here!\n").unwrap();
        let tag = args.call_info.name_tag.clone();
        Ok(OutputStream::one(ReturnSuccess::value(UntaggedValue::string("success!").into_value(tag))))
    }
}