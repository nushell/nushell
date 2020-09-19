use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
#[cfg(feature = "rich-benchmark")]
use heim::cpu::time;
use nu_errors::ShellError;
use nu_protocol::{
    format_duration, hir::Block, Dictionary, Signature, SyntaxShape, UntaggedValue, Value,
};
use std::convert::TryInto;
use std::time::{Duration, Instant};

pub struct Benchmark;

#[derive(Deserialize, Debug)]
struct BenchmarkArgs {
    block: Block,
    passthrough: bool,
}

#[async_trait]
impl WholeStreamCommand for Benchmark {
    fn name(&self) -> &str {
        "benchmark"
    }

    fn signature(&self) -> Signature {
        Signature::build("benchmark")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run and benchmark",
            )
            .switch(
                "passthrough",
                "Display the benchmark results and pass through the block's output",
                Some('p'),
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
        vec![
            Example {
                description: "Benchmarks a command within a block",
                example: "benchmark { sleep 500ms }",
                result: None,
            },
            Example {
                description: "Benchmarks a command within a block and passes its output through",
                example: "echo 45 | benchmark --passthrough { sleep 500ms }",
                result: Some(vec![UntaggedValue::int(45).into()]),
            },
        ]
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
    let (BenchmarkArgs { block, passthrough }, input) = raw_args.process(&registry).await?;

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
    let output = result?.into_vec().await;

    #[cfg(feature = "rich-benchmark")]
    let end = time().await;

    let end_time = Instant::now();
    context.clear_errors();

    // return basic runtime
    #[cfg(not(feature = "rich-benchmark"))]
    {
        let mut indexmap = IndexMap::with_capacity(1);

        let real_time = into_big_int(end_time - start_time);
        indexmap.insert("real time".to_string(), real_time);
        benchmark_output(indexmap, output, passthrough, &tag)
    }
    // return advanced stats
    #[cfg(feature = "rich-benchmark")]
    if let (Ok(start), Ok(end)) = (start, end) {
        let mut indexmap = IndexMap::with_capacity(4);

        let real_time = into_big_int(end_time - start_time);
        indexmap.insert("real time".to_string(), real_time);

        let user_time = into_big_int(end.user() - start.user());
        indexmap.insert("user time".to_string(), user_time);

        let system_time = into_big_int(end.system() - start.system());
        indexmap.insert("system time".to_string(), system_time);

        let idle_time = into_big_int(end.idle() - start.idle());
        indexmap.insert("idle time".to_string(), idle_time);

        benchmark_output(indexmap, output, passthrough, &tag)
    } else {
        Err(ShellError::untagged_runtime_error(
            "Could not retreive CPU time",
        ))
    }
}

fn into_big_int<T: TryInto<Duration>>(time: T) -> BigInt {
    time.try_into()
        .unwrap_or_else(|_| Duration::new(0, 0))
        .as_nanos()
        .into()
}

fn benchmark_output<T, Output>(
    indexmap: IndexMap<String, BigInt>,
    block_output: Output,
    passthrough: bool,
    tag: T,
) -> Result<OutputStream, ShellError>
where
    T: Into<Tag> + Copy,
    Output: Into<OutputStream>,
{
    if passthrough {
        for (key, time) in indexmap {
            println!("{}:\t{}", key, format_duration(&time));
        }
        Ok(block_output.into())
    } else {
        let value = UntaggedValue::Row(Dictionary::from(
            indexmap
                .into_iter()
                .map(|(k, v)| (k, UntaggedValue::duration(v).into_value(tag)))
                .collect::<IndexMap<String, Value>>(),
        ))
        .into_value(tag);
        Ok(OutputStream::one(value))
    }
}
