use crate::prelude::*;
// #[cfg(feature = "rich-benchmark")]
// use heim::cpu::time;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::{
        Block, CapturedBlock, ClassifiedCommand, ExternalRedirection, Group, InternalCommand,
        Pipeline,
    },
    Dictionary, Signature, SyntaxShape, UntaggedValue, Value,
};
use rand::{
    distributions::Alphanumeric,
    prelude::{thread_rng, Rng},
};
use std::time::Instant;

pub struct Benchmark;

#[derive(Debug)]
struct BenchmarkArgs {
    block: CapturedBlock,
    passthrough: Option<CapturedBlock>,
}

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
            .named(
                "passthrough",
                SyntaxShape::Block,
                "Display the benchmark results and pass through the block's output",
                Some('p'),
            )
    }

    fn usage(&self) -> &str {
        "Runs a block and returns the time it took to execute it."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        benchmark(args)
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
                example: "echo 45 | benchmark { sleep 500ms } --passthrough {}",
                result: Some(vec![UntaggedValue::int(45).into()]),
            },
        ]
    }
}

fn benchmark(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.args.span;
    let mut context = args.context.clone();
    let scope = args.scope().clone();

    let cmd_args = BenchmarkArgs {
        block: args.req(0)?,
        passthrough: args.get_flag("passthrough")?,
    };

    let env = scope.get_env_vars();
    let name = generate_free_name(&env);

    scope.add_env_var(name, generate_random_env_value());

    let start_time = Instant::now();

    // #[cfg(feature = "rich-benchmark")]
    // let start = time();

    context.scope.enter_scope();
    let result = run_block(
        &cmd_args.block.block,
        &context,
        args.input,
        ExternalRedirection::StdoutAndStderr,
    );

    context.scope.exit_scope();
    let output = result?.into_vec();

    // #[cfg(feature = "rich-benchmark")]
    // let end = time();

    let end_time = Instant::now();
    context.clear_errors();

    // return basic runtime
    //#[cfg(not(feature = "rich-benchmark"))]
    {
        let mut indexmap = IndexMap::with_capacity(1);

        let real_time = (end_time - start_time).as_nanos() as i64;
        indexmap.insert("real time".to_string(), real_time);
        benchmark_output(indexmap, output, cmd_args.passthrough, &tag, &mut context)
    }
    // return advanced stats
    // #[cfg(feature = "rich-benchmark")]
    // if let (Ok(start), Ok(end)) = (start, end) {
    //     let mut indexmap = IndexMap::with_capacity(4);

    //     let real_time = into_big_int(end_time - start_time);
    //     indexmap.insert("real time".to_string(), real_time);

    //     let user_time = into_big_int(end.user() - start.user());
    //     indexmap.insert("user time".to_string(), user_time);

    //     let system_time = into_big_int(end.system() - start.system());
    //     indexmap.insert("system time".to_string(), system_time);

    //     let idle_time = into_big_int(end.idle() - start.idle());
    //     indexmap.insert("idle time".to_string(), idle_time);

    //     benchmark_output(indexmap, output, passthrough, &tag, &mut context)
    // } else {
    //     Err(ShellError::untagged_runtime_error(
    //         "Could not retrieve CPU time",
    //     ))
    // }
}

fn benchmark_output<T, Output>(
    indexmap: IndexMap<String, i64>,
    block_output: Output,
    passthrough: Option<CapturedBlock>,
    tag: T,
    context: &mut EvaluationContext,
) -> Result<OutputStream, ShellError>
where
    T: Into<Tag> + Copy,
    Output: Into<OutputStream>,
{
    let value = UntaggedValue::Row(Dictionary::from(
        indexmap
            .into_iter()
            .map(|(k, v)| (k, UntaggedValue::duration(v).into_value(tag)))
            .collect::<IndexMap<String, Value>>(),
    ))
    .into_value(tag);

    if let Some(time_block) = passthrough {
        let benchmark_output = InputStream::one(value);

        // add autoview for an empty block
        let time_block = add_implicit_autoview(time_block.block);

        context.scope.enter_scope();
        let result = run_block(
            &time_block,
            context,
            benchmark_output,
            ExternalRedirection::StdoutAndStderr,
        );
        context.scope.exit_scope();
        result?;
        context.clear_errors();

        Ok(block_output.into())
    } else {
        let benchmark_output = OutputStream::one(value);
        Ok(benchmark_output)
    }
}

fn add_implicit_autoview(mut block: Arc<Block>) -> Arc<Block> {
    if let Some(block) = std::sync::Arc::<nu_protocol::hir::Block>::get_mut(&mut block) {
        if block.block.is_empty() {
            let group = Group::new(
                vec![{
                    let mut commands = Pipeline::new(block.span);
                    commands.push(ClassifiedCommand::Internal(InternalCommand::new(
                        "autoview".to_string(),
                        block.span,
                        block.span,
                    )));
                    commands
                }],
                block.span,
            );
            block.push(group);
        }
    }
    block
}

fn generate_random_env_value() -> String {
    let mut thread_rng = thread_rng();
    let len = thread_rng.gen_range(1, 16 * 1024);
    thread_rng.sample_iter(&Alphanumeric).take(len).collect()
}

fn generate_free_name(env: &indexmap::IndexMap<String, String>) -> String {
    let mut thread_rng = thread_rng();
    loop {
        let candidate_name = format!("NU_RANDOM_VALUE_{}", thread_rng.gen::<usize>());
        if !env.contains_key(&candidate_name) {
            return candidate_name;
        }
    }
}
