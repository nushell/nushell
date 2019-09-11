use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

pub struct Last;

#[derive(Deserialize)]
pub struct LastArgs {
    amount: Tagged<u64>,
}

impl WholeStreamCommand for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last").required("amount", SyntaxType::Number)
    }

    fn usage(&self) -> &str {
        "Show only the last number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, last)?.run()
    }
}

fn last(
    LastArgs { amount }: LastArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream_block! {
        let v: Vec<_> = context.input.into_vec().await;
        let k = v.len() - (*amount as usize);
        for x in v[k..].iter() {
            let y: Tagged<Value> = x.clone();
            yield ReturnSuccess::value(y)
        }
    };
    Ok(stream.to_output_stream())
}
