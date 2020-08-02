use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

use chrono::prelude::*;

pub struct Benchmark;

#[derive(Deserialize, Debug)]
struct BenchmarkArgs {
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for Benchmark {
    fn name(&self) -> &str {
        "benchmark"
    }

    fn signature(&self) -> Signature {
        Signature::build("benchmark").required(
            "block",
            SyntaxShape::Block,
            "the block to run and benchmark",
        )
    }

    fn usage(&self) -> &str {
        "Runs a block and return the time it took to do execute it. Eg) benchmark { echo $nu.env.NAME }"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        benchmark(args, registry).await
    }
}

async fn benchmark(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let mut context = Context::from_raw(&raw_args, &registry);
    let scope = raw_args.call_info.scope.clone();
    let (BenchmarkArgs { block }, input) = raw_args.process(&registry).await?;

    let start_time: chrono::DateTime<_> = Utc::now();

    let result = run_block(
        &block,
        &mut context,
        input,
        &scope.it,
        &scope.vars,
        &scope.env,
    )
    .await;

    let _ = result?.drain_vec().await;
    let run_duration: chrono::Duration = Utc::now().signed_duration_since(start_time);

    context.clear_errors();

    let output = Ok(ReturnSuccess::Value(Value {
        value: UntaggedValue::Primitive(Primitive::from(run_duration)),
        tag: Tag::from(block.span),
    }));

    Ok(OutputStream::from(vec![output]))
}
