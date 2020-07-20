use futures::executor::block_on;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::hir::ClassifiedBlock;
use nu_protocol::{ShellTypeName, Value};

use crate::commands::classified::block::run_block;
use crate::commands::{whole_stream_command, BuildString, Echo, StrCollect};
use crate::context::Context;
use crate::stream::InputStream;
use crate::WholeStreamCommand;

pub fn test(cmd: impl WholeStreamCommand + 'static) {
    let examples = cmd.examples();
    let mut base_context = Context::basic().expect("could not create basic context");

    base_context.add_commands(vec![
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(cmd),
        whole_stream_command(StrCollect),
    ]);

    for example in examples {
        let mut ctx = base_context.clone();
        let block = parse_line(example.example, &mut ctx).expect("failed to parse example");
        if let Some(expected) = example.result {
            let result = block_on(evaluate_block(block, &mut ctx)).expect("failed to run example");

            let errors = ctx.get_errors();

            assert!(
                errors.is_empty(),
                "errors while running command.\ncommand: {}\nerrors: {:?}",
                example.example,
                errors
            );

            assert!(expected.len() == result.len(), "example command produced unexpected number of results.\ncommand: {}\nexpected number: {}\nactual: {}",
                example.example,
                expected.len(),
                result.len(),);

            assert!(
                expected
                    .iter()
                    .zip(result.iter())
                    .all(|(e, a)| values_equal(e, a)),
                "example command produced unexpected result.\ncommand: {}\nexpected: {:?}\nactual: {:?}",
                example.example,
                expected,
                result,
            );
        }
    }
}

/// Parse and run a nushell pipeline
fn parse_line(line: &'static str, ctx: &mut Context) -> Result<ClassifiedBlock, ShellError> {
    let line = if line.ends_with('\n') {
        &line[..line.len() - 1]
    } else {
        line
    };

    let lite_result = nu_parser::lite_parse(&line, 0)?;

    // TODO ensure the command whose examples we're testing is actually in the pipeline
    let mut classified_block = nu_parser::classify_block(&lite_result, ctx.registry());
    classified_block.block.expand_it_usage();
    Ok(classified_block)
}

async fn evaluate_block(
    block: ClassifiedBlock,
    ctx: &mut Context,
) -> Result<Vec<Value>, ShellError> {
    let input_stream = InputStream::empty();
    let env = ctx.get_env();

    Ok(run_block(
        &block.block,
        ctx,
        input_stream,
        &Value::nothing(),
        &IndexMap::new(),
        &env,
    )
    .await?
    .into_vec()
    .await)
}

// TODO probably something already available to do this
// TODO perhaps better panic messages when things don't compare

// Deep value comparisons that ignore tags
fn values_equal(expected: &Value, actual: &Value) -> bool {
    use nu_protocol::UntaggedValue::*;

    match (&expected.value, &actual.value) {
        (Primitive(e), Primitive(a)) => e == a,
        (Row(e), Row(a)) => {
            if e.entries.len() != a.entries.len() {
                return false;
            }

            e.entries
                .iter()
                .zip(a.entries.iter())
                .all(|((ek, ev), (ak, av))| ek == ak && values_equal(ev, av))
        }
        (Table(e), Table(a)) => e.iter().zip(a.iter()).all(|(e, a)| values_equal(e, a)),
        (e, a) => unimplemented!("{} {}", e.type_name(), a.type_name()),
    }
}
