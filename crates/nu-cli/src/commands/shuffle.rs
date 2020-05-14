use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, SyntaxShape, Value};
use nu_source::Tagged;

use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Shuffle;

#[derive(Deserialize)]
pub struct Arguments {
    #[serde(rename = "num")]
    limit: Option<Tagged<u64>>,
}

impl WholeStreamCommand for Shuffle {
    fn name(&self) -> &str {
        "shuffle"
    }

    fn signature(&self) -> Signature {
        Signature::build("shuffle").named(
            "num",
            SyntaxShape::Int,
            "Limit `num` number of rows",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "Shuffle rows randomly."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        shuffle(args, registry)
    }
}

fn shuffle(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let mut input = args.input;
        let Arguments { limit } = args.process_raw(&registry).await?;
        let mut values: Vec<Value> = input.collect().await;

        let out = if let Some(n) = limit {
            let (shuffled, _) = values.partial_shuffle(&mut thread_rng(), *n as usize);
            shuffled.to_vec()
        } else {
            values.shuffle(&mut thread_rng());
            values.clone()
        };

        for val in out.into_iter() {
            yield ReturnSuccess::value(val);
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}
