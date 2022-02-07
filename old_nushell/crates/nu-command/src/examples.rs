mod sample;

mod double_echo;
mod double_ls;
mod stub_generate;

use double_echo::Command as DoubleEcho;
use double_ls::Command as DoubleLs;
use stub_generate::{mock_path, Command as StubOpen};

use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::{ClassifiedBlock, ExternalRedirection};
use nu_protocol::{ShellTypeName, Value};
use nu_source::AnchorLocation;

#[cfg(feature = "dataframe")]
use crate::commands::{
    DataFrameDropNulls, DataFrameGroupBy, DataFrameIsNull, DataFrameShift, DataFrameToDF,
    DataFrameWithColumn, StrToDatetime,
};

use crate::commands::{
    Append, BuildString, Collect, Each, Echo, First, Get, If, IntoInt, Keep, Last, Let, Math,
    MathMode, Nth, Select, StrCollect, Wrap,
};
use nu_engine::{run_block, whole_stream_command, Command, EvaluationContext, WholeStreamCommand};
use nu_stream::InputStream;

pub fn test_examples(cmd: Command) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = EvaluationContext::basic();

    base_context.add_commands(vec![
        // Command Doubles
        whole_stream_command(DoubleLs {}),
        // Minimal restricted commands to aid in testing
        whole_stream_command(Append {}),
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(If {}),
        whole_stream_command(IntoInt {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Collect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &ctx)?;

        if let Some(expected) = &sample_pipeline.result {
            let result = evaluate_block(block, &mut ctx)?;

            ctx.with_errors(|reasons| reasons.iter().cloned().next())
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

            for (e, a) in expected.iter().zip(&result) {
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

    let base_context = EvaluationContext::basic();

    base_context.add_commands(vec![
        whole_stream_command(Math),
        whole_stream_command(MathMode {}),
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Let {}),
        whole_stream_command(cmd),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Collect),
        whole_stream_command(Wrap),
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &ctx)?;

        if let Some(expected) = &sample_pipeline.result {
            let start = std::time::Instant::now();
            let result = evaluate_block(block, &mut ctx)?;

            println!("input: {}", sample_pipeline.example);
            println!("result: {:?}", result);
            println!("done: {:?}", start.elapsed());

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

            for (e, a) in expected.iter().zip(&result) {
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

#[cfg(feature = "dataframe")]
pub fn test_dataframe(cmd: impl WholeStreamCommand + 'static) -> Result<(), ShellError> {
    use nu_protocol::UntaggedValue;

    let examples = cmd.examples();

    let base_context = EvaluationContext::basic();

    base_context.add_commands(vec![
        whole_stream_command(cmd),
        // Commands used with dataframe
        whole_stream_command(DataFrameToDF),
        whole_stream_command(DataFrameShift),
        whole_stream_command(DataFrameIsNull),
        whole_stream_command(DataFrameGroupBy),
        whole_stream_command(DataFrameWithColumn),
        whole_stream_command(DataFrameDropNulls),
        // Base commands for context
        whole_stream_command(Math),
        whole_stream_command(MathMode {}),
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Collect),
        whole_stream_command(Wrap),
        whole_stream_command(StrToDatetime),
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        println!("{:?}", &sample_pipeline.example);
        let block = parse_line(sample_pipeline.example, &ctx)?;

        if let Some(expected) = &sample_pipeline.result {
            let start = std::time::Instant::now();
            let result = evaluate_block(block, &mut ctx)?;

            println!("input: {}", sample_pipeline.example);
            println!("result: {:?}", result);
            println!("done: {:?}", start.elapsed());

            let value = match result.get(0) {
                Some(v) => v,
                None => panic!(
                    "Unable to extract a value after parsing example: {}",
                    sample_pipeline.example
                ),
            };

            let df = match &value.value {
                UntaggedValue::DataFrame(df) => df,
                _ => panic!(
                    "Unable to extract dataframe from parsed example: {}",
                    sample_pipeline.example
                ),
            };

            let expected = match expected.get(0) {
                Some(v) => v,
                None => panic!("Empty vector in result example"),
            };

            let df_expected = match &expected.value {
                UntaggedValue::DataFrame(df) => df,
                _ => panic!("Unable to extract dataframe from example result"),
            };

            println!("expected: {:?}", df_expected);

            assert_eq!(df, df_expected)
        }
    }

    Ok(())
}

pub fn test_anchors(cmd: Command) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = EvaluationContext::basic();

    base_context.add_commands(vec![
        // Minimal restricted commands to aid in testing
        whole_stream_command(StubOpen {}),
        whole_stream_command(DoubleEcho {}),
        whole_stream_command(DoubleLs {}),
        whole_stream_command(Append {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(If {}),
        whole_stream_command(IntoInt {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Collect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let pipeline_with_anchor = format!("stub open --path | {}", sample_pipeline.example);

        let mut ctx = base_context.clone();

        let block = parse_line(&pipeline_with_anchor, &ctx)?;

        if sample_pipeline.result.is_some() {
            let result = evaluate_block(block, &mut ctx)?;

            ctx.with_errors(|reasons| reasons.iter().cloned().next())
                .map_or(Ok(()), Err)?;

            for actual in &result {
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

    let (lite_result, err) = nu_parser::lex(line, 0, nu_parser::NewlineMode::Normal);
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

fn evaluate_block(
    block: ClassifiedBlock,
    ctx: &mut EvaluationContext,
) -> Result<Vec<Value>, ShellError> {
    let input_stream = InputStream::empty();

    ctx.scope.enter_scope();

    let result = run_block(&block.block, ctx, input_stream, ExternalRedirection::Stdout);

    ctx.scope.exit_scope();

    let result = result?.drain_vec();
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
                .zip(&a.entries)
                .all(|((ek, ev), (ak, av))| ek == ak && values_equal(ev, av))
        }
        (Table(e), Table(a)) => e.iter().zip(a).all(|(e, a)| values_equal(e, a)),
        (e, a) => unimplemented!("{} {}", e.type_name(), a.type_name()),
    }
}

fn is_anchor_carried(actual: &Value, anchor: AnchorLocation) -> bool {
    actual.tag.anchor() == Some(anchor)
}
