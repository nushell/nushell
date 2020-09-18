use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
#[cfg(feature = "rich-benchmark")]
use heim::cpu::time;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Dictionary, Signature, SyntaxShape, UntaggedValue, Value};
use std::convert::TryInto;
use std::time::{Duration, Instant};

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
        "Runs a block and returns the time it took to execute it"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        benchmark(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Benchmarks a command within a block",
            example: "benchmark { sleep 500ms }",
            result: None,
        }]
    }
}

async fn benchmark(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let tag = raw_args.call_info.args.span;
    let mut context = Context::from_raw(&raw_args, &registry);
    let scope = raw_args.call_info.scope.clone();
    let (BenchmarkArgs { block }, input) = raw_args.process(&registry).await?;

    let start_time = Instant::now();

    #[cfg(feature = "rich-benchmark")]
    let start = time().await;

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

    #[cfg(feature = "rich-benchmark")]
    let end = time().await;

    let end_time = Instant::now();
    context.clear_errors();

    if let (Ok(start), Ok(end)) = (start, end) {
        fn into_value<T: TryInto<Duration>>(time: T, tag: &Span) -> Value {
            UntaggedValue::duration(
                time.try_into()
                    .unwrap_or_else(|_| Duration::new(0, 0))
                    .as_nanos()
                    .into(),
            )
            .into_value(tag)
        }

        let mut indexmap = IndexMap::with_capacity(4);

        let real_time = into_value(end_time - start_time, &tag);
        indexmap.insert("real time".to_string(), real_time);

        #[cfg(feature = "rich-benchmark")]
        {
            let user_time = into_value(end.user() - start.user(), &tag);
            indexmap.insert("user time".to_string(), user_time);

            let system_time = into_value(end.system() - start.system(), &tag);
            indexmap.insert("system time".to_string(), system_time);

            let idle_time = into_value(end.idle() - start.idle(), &tag);
            indexmap.insert("idle time".to_string(), idle_time);
        }

        let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
        Ok(OutputStream::one(value))
    } else {
        Err(ShellError::untagged_runtime_error(
            "Could not retreive CPU time",
        ))
    }
}
