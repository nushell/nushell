use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::ClassifiedBlock;
use nu_protocol::{
    Primitive, ReturnSuccess, ShellTypeName, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{AnchorLocation, TaggedItem};

use crate::prelude::*;

use num_bigint::BigInt;

use crate::commands::classified::block::run_block;
use crate::commands::command::CommandArgs;
use crate::commands::{
    whole_stream_command, BuildString, Command, Each, Echo, First, Get, Keep, Last, Nth, Set,
    StrCollect, WholeStreamCommand, Wrap,
};
use crate::evaluation_context::EvaluationContext;
use nu_stream::{InputStream, OutputStream};

use async_trait::async_trait;
use futures::executor::block_on;
use serde::Deserialize;

pub fn test_examples(cmd: Command) -> Result<(), ShellError> {
    let examples = cmd.examples();

    let base_context = EvaluationContext::basic()?;

    base_context.add_commands(vec![
        // Mocks
        whole_stream_command(MockLs {}),
        // Minimal restricted commands to aid in testing
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Set {}),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &mut ctx)?;

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

    let base_context = EvaluationContext::basic()?;

    base_context.add_commands(vec![
        whole_stream_command(Echo {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Set {}),
        whole_stream_command(cmd),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
    ]);

    for sample_pipeline in examples {
        let mut ctx = base_context.clone();

        let block = parse_line(sample_pipeline.example, &mut ctx)?;

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

    let base_context = EvaluationContext::basic()?;

    base_context.add_commands(vec![
        // Minimal restricted commands to aid in testing
        whole_stream_command(MockCommand {}),
        whole_stream_command(MockEcho {}),
        whole_stream_command(MockLs {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Set {}),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
        cmd,
    ]);

    for sample_pipeline in examples {
        let pipeline_with_anchor = format!("mock --open --path | {}", sample_pipeline.example);

        let mut ctx = base_context.clone();

        let block = parse_line(&pipeline_with_anchor, &mut ctx)?;
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
    let (lite_result, err) = nu_parser::group(lite_result);
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

#[derive(Deserialize)]
struct Arguments {
    path: Option<bool>,
    open: bool,
}

struct MockCommand;

#[async_trait]
impl WholeStreamCommand for MockCommand {
    fn name(&self) -> &str {
        "mock"
    }

    fn signature(&self) -> Signature {
        Signature::build("mock")
            .switch("open", "fake opening sources", Some('o'))
            .switch("path", "file open", Some('p'))
    }

    fn usage(&self) -> &str {
        "Generates tables and metadata that mimics behavior of real commands in controlled ways."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();

        let (
            Arguments {
                path: mocked_path,
                open: open_mock,
            },
            _input,
        ) = args.process().await?;

        let out = UntaggedValue::string("Yehuda Katz in Ecuador");

        if open_mock {
            if let Some(true) = mocked_path {
                return Ok(OutputStream::one(Ok(ReturnSuccess::Value(Value {
                    value: out,
                    tag: Tag {
                        anchor: Some(mock_path()),
                        span: name_tag.span,
                    },
                }))));
            }
        }

        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            out.into_value(name_tag),
        ))))
    }
}

struct MockEcho;

#[derive(Deserialize)]
struct MockEchoArgs {
    pub rest: Vec<Value>,
}

#[async_trait]
impl WholeStreamCommand for MockEcho {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Mock echo."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();
        let (MockEchoArgs { rest }, input) = args.process().await?;

        let mut base_value = UntaggedValue::string("Yehuda Katz in Ecuador").into_value(name_tag);
        let input: Vec<Value> = input.collect().await;

        if let Some(first) = input.get(0) {
            base_value = first.clone()
        }

        let stream = rest.into_iter().map(move |i| {
            let base_value = base_value.clone();
            match i.as_string() {
                Ok(s) => OutputStream::one(Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag: base_value.tag,
                }))),
                _ => match i {
                    Value {
                        value: UntaggedValue::Table(table),
                        ..
                    } => {
                        if table.len() == 1 && table[0].is_table() {
                            let mut values: Vec<Value> =
                                table[0].table_entries().map(Clone::clone).collect();

                            for v in values.iter_mut() {
                                v.tag = base_value.tag();
                            }

                            let subtable =
                                vec![UntaggedValue::Table(values).into_value(base_value.tag())];

                            futures::stream::iter(subtable.into_iter().map(ReturnSuccess::value))
                                .to_output_stream()
                        } else {
                            futures::stream::iter(
                                table
                                    .into_iter()
                                    .map(move |mut v| {
                                        v.tag = base_value.tag();
                                        v
                                    })
                                    .map(ReturnSuccess::value),
                            )
                            .to_output_stream()
                        }
                    }
                    _ => OutputStream::one(Ok(ReturnSuccess::Value(Value {
                        value: i.value.clone(),
                        tag: base_value.tag,
                    }))),
                },
            }
        });

        Ok(futures::stream::iter(stream).flatten().to_output_stream())
    }
}

struct MockLs;

#[async_trait]
impl WholeStreamCommand for MockLs {
    fn name(&self) -> &str {
        "ls"
    }

    fn signature(&self) -> Signature {
        Signature::build("ls")
    }

    fn usage(&self) -> &str {
        "Mock ls."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();

        let mut base_value =
            UntaggedValue::string("Andrés N. Robalino in Portland").into_value(name_tag);
        let input: Vec<Value> = args.input.collect().await;

        if let Some(first) = input.get(0) {
            base_value = first.clone()
        }

        Ok(futures::stream::iter(
            file_listing()
                .iter()
                .map(|row| Value {
                    value: row.value.clone(),
                    tag: base_value.tag.clone(),
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(ReturnSuccess::value),
        )
        .to_output_stream())
    }
}

fn int(s: impl Into<BigInt>) -> Value {
    UntaggedValue::int(s).into_untagged_value()
}

fn string(input: impl Into<String>) -> Value {
    UntaggedValue::string(input.into()).into_untagged_value()
}

fn date(input: impl Into<String>) -> Value {
    let key = input.into().tagged_unknown();
    crate::value::Date::naive_from_str(key.borrow_tagged())
        .expect("date from string failed")
        .into_untagged_value()
}

fn file_listing() -> Vec<Value> {
    vec![
        row! {
               "name".to_string() => string("Andrés.txt"),
               "type".to_string() =>       string("File"),
           "chickens".to_string() =>              int(10),
           "modified".to_string() =>   date("2019-07-23")
        },
        row! {
               "name".to_string() =>   string("Jonathan"),
               "type".to_string() =>        string("Dir"),
           "chickens".to_string() =>               int(5),
           "modified".to_string() =>   date("2019-07-23")
        },
        row! {
               "name".to_string() =>  string("Andrés.txt"),
               "type".to_string() =>        string("File"),
           "chickens".to_string() =>               int(20),
           "modified".to_string() =>    date("2019-09-24")
        },
        row! {
               "name".to_string() =>      string("Yehuda"),
               "type".to_string() =>         string("Dir"),
           "chickens".to_string() =>                int(4),
           "modified".to_string() =>    date("2019-09-24")
        },
    ]
}

fn mock_path() -> AnchorLocation {
    let path = String::from("path/to/las_best_arepas_in_the_world.txt");

    AnchorLocation::File(path)
}
