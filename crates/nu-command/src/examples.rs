mod sample;

mod double_echo;
mod double_ls;
mod stub_generate;

use double_echo::Command as DoubleEcho;
use double_ls::Command as DoubleLs;
use stub_generate::{mock_path, Command as StubOpen};

use nu_engine::basic_evaluation_context;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::ClassifiedBlock;
use nu_protocol::{ShellTypeName, Value};
use nu_source::AnchorLocation;

use crate::commands::{
    Append, BuildString, Each, Echo, First, Get, Keep, Last, Let, Nth, Select, StrCollect, Wrap,
};
use nu_engine::{run_block, whole_stream_command, Command, EvaluationContext, WholeStreamCommand};
use nu_stream::InputStream;

use futures::executor::block_on;

pub fn test_examples(cmd: Command) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = basic_evaluation_context()?;

    base_context.add_commands(vec![
        // Command Doubles
        whole_stream_command(DoubleLs {}),
        // Minimal restricted commands to aid in testing
        whole_stream_command(Append {}),
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &ctx)?;

        println!("{:#?}", block);

        if let Some(expected) = &sample_pipeline.result {
            let result = block_on(evaluate_block(block, &mut ctx))?;

            ctx.with_errors(|reasons| reasons.iter().cloned().take(1).next())
                .map_or(Ok(()), Err)?;

            if expected.len() != result.len() {
                let rows_returned =
                    format!("expected: {}\nactual: {}", expected.len(), result.len());
                let failed_call = format!("command: {}\n", sample_pipeline.example);

                panic!(
                    "example command produced unexpected number of results.\n {} {}",
                    failed_call, rows_returned
                );
            }

            for (e, a) in expected.iter().zip(result.iter()) {
                if !values_equal(e, a) {
                    let row_errored = format!("expected: {:#?}\nactual: {:#?}", e, a);
                    let failed_call = format!("command: {}\n", sample_pipeline.example);

                    panic!(
                        "example command produced unexpected result.\n {} {}",
                        failed_call, row_errored
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn test(cmd: impl WholeStreamCommand + 'static) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = basic_evaluation_context()?;

    base_context.add_commands(vec![
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Let {}),
        whole_stream_command(cmd),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &ctx)?;

        if let Some(expected) = &sample_pipeline.result {
            let result = block_on(evaluate_block(block, &mut ctx))?;

            ctx.with_errors(|reasons| reasons.iter().cloned().take(1).next())
                .map_or(Ok(()), Err)?;

            if expected.len() != result.len() {
                let rows_returned =
                    format!("expected: {}\nactual: {}", expected.len(), result.len());
                let failed_call = format!("command: {}\n", sample_pipeline.example);

                panic!(
                    "example command produced unexpected number of results.\n {} {}",
                    failed_call, rows_returned
                );
            }

            for (e, a) in expected.iter().zip(result.iter()) {
                if !values_equal(e, a) {
                    let row_errored = format!("expected: {:#?}\nactual: {:#?}", e, a);
                    let failed_call = format!("command: {}\n", sample_pipeline.example);

                    panic!(
                        "example command produced unexpected result.\n {} {}",
                        failed_call, row_errored
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn test_anchors(cmd: Command) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = basic_evaluation_context()?;

    base_context.add_commands(vec![
        // Minimal restricted commands to aid in testing
        whole_stream_command(StubOpen {}),
        whole_stream_command(DoubleEcho {}),
        whole_stream_command(DoubleLs {}),
        whole_stream_command(Append {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let pipeline_with_anchor = format!("stub open --path | {}", sample_pipeline.example);

        let mut ctx = base_context.clone();

        let block = parse_line(&pipeline_with_anchor, &ctx)?;

        if sample_pipeline.result.is_some() {
            let result = block_on(evaluate_block(block, &mut ctx))?;

            ctx.with_errors(|reasons| reasons.iter().cloned().take(1).next())
                .map_or(Ok(()), Err)?;

            for actual in result.iter() {
                if !is_anchor_carried(actual, mock_path()) {
                    let failed_call = format!("command: {}\n", pipeline_with_anchor);

                    panic!(
                        "example command didn't carry anchor tag correctly.\n {} {:#?} {:#?}",
                        failed_call,
                        actual,
                        mock_path()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Parse and run a nushell pipeline
fn parse_line(line: &str, ctx: &EvaluationContext) -> Result<ClassifiedBlock, ShellError> {
    //FIXME: do we still need this?
    let line = if let Some(line) = line.strip_suffix('\n') {
        line
    } else {
        line
    };

    let (lite_result, err) = nu_parser::lex(&line, 0);
    if let Some(err) = err {
        return Err(err.into());
    }
    let (lite_result, err) = nu_parser::parse_block(lite_result);
    if let Some(err) = err {
        return Err(err.into());
    }

    // TODO ensure the command whose examples we're testing is actually in the pipeline
    let (block, err) = nu_parser::classify_block(&lite_result, &ctx.scope);
    Ok(ClassifiedBlock { block, failed: err })
}

async fn evaluate_block(
    block: ClassifiedBlock,
    ctx: &mut EvaluationContext,
) -> Result<Vec<Value>, ShellError> {
    let input_stream = InputStream::empty();
    let env = ctx.get_env();

    ctx.scope.enter_scope();
    ctx.scope.add_env(env);

    let result = run_block(&block.block, ctx, input_stream).await;

    ctx.scope.exit_scope();

    let result = result?.drain_vec().await;
    Ok(result)
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

fn is_anchor_carried(actual: &Value, anchor: AnchorLocation) -> bool {
    actual.tag.anchor() == Some(anchor)
}
