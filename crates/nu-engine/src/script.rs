use crate::{maybe_print_errors, path::canonicalize, run_block};
use crate::{MaybeTextCodec, StringOrBinary};
use futures::StreamExt;
use futures_codec::FramedRead;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::{
    hir::{
        Call, ClassifiedCommand, Expression, InternalCommand, Literal, NamedArguments,
        SpannedExpression,
    },
    NuScript, ReturnSuccess, RunScriptOptions,
};
use nu_protocol::{Primitive, UntaggedValue, Value};
use nu_stream::{InputStream, ToInputStream};

use crate::EvaluationContext;
use log::{debug, trace};
use nu_source::{Span, Tag, Text};
use std::path::Path;
use std::{iter::Iterator, sync::atomic::Ordering};

#[derive(Debug)]
pub enum LineResult {
    Success(String),
    Error(String, ShellError),
    Break,
    CtrlC,
    CtrlD,
    ClearHistory,
}

//TODO is this still needed
fn chomp_newline(s: &str) -> &str {
    if let Some(s) = s.strip_suffix('\n') {
        s
    } else {
        s
    }
}

/// Runs script `script` configurable by `options`
/// All errors are printed out.
pub async fn run_script(script: NuScript, options: &RunScriptOptions, ctx: &EvaluationContext) {
    let code = match script.get_code() {
        Ok(code) => code,
        Err(e) => {
            ctx.host.lock().print_err(e, &Text::from("".to_string()));
            return;
        }
    };

    if let Err(e) = setup_shell(options, ctx) {
        ctx.host.lock().print_err(e, &Text::from("".to_string()));
        return;
    }

    if !options.use_existing_scope {
        ctx.scope.enter_scope()
    }

    let line_result = process_script(&code, options, ctx).await;
    evaluate_line_result(line_result, options, ctx).await;

    if !options.use_existing_scope {
        ctx.scope.exit_scope()
    }

    //Leave script shell
    ctx.shell_manager.remove_at_current();
}

fn setup_shell(options: &RunScriptOptions, ctx: &EvaluationContext) -> Result<(), ShellError> {
    //Switch to correct shell
    if options.cli_mode {
        ctx.shell_manager.enter_cli_mode()?;
    } else {
        ctx.shell_manager.enter_script_mode()?;
    }

    //Switch to cwd if given
    if let Some(path) = &options.with_cwd {
        ctx.shell_manager
            .set_path(path.to_string_lossy().to_string());
    }

    Ok(())
}

async fn evaluate_line_result(
    line_result: LineResult,
    options: &RunScriptOptions,
    context: &EvaluationContext,
) {
    match line_result {
        LineResult::Success(line) => {
            let error_code = {
                let errors = context.current_errors.clone();
                let errors = errors.lock();

                if errors.len() > 0 {
                    1
                } else {
                    0
                }
            };

            maybe_print_errors(&context, Text::from(line));
            if error_code != 0 && options.exit_on_error {
                std::process::exit(error_code);
            }
        }

        LineResult::Error(line, err) => {
            context
                .host
                .lock()
                .print_err(err, &Text::from(line.clone()));

            maybe_print_errors(&context, Text::from(line));
            if options.exit_on_error {
                std::process::exit(1);
            }
        }

        _ => {}
    }
}

/// Process the line by parsing the text to turn it into commands, classify those commands so that we understand what is being called in the pipeline, and then run this pipeline
pub async fn process_script(
    script_text: &str,
    options: &RunScriptOptions,
    ctx: &EvaluationContext,
) -> LineResult {
    if script_text.trim() == "" {
        LineResult::Success(script_text.to_string())
    } else {
        let line = chomp_newline(script_text);

        let (block, err) = nu_parser::parse(&line, options.span_offset, &ctx.scope);

        debug!("{:#?}", block);
        //println!("{:#?}", pipeline);

        if let Some(failure) = err {
            return LineResult::Error(line.to_string(), failure.into());
        }

        // There's a special case to check before we process the pipeline:
        // If we're giving a path by itself
        // ...and it's not a command in the path
        // ...and it doesn't have any arguments
        // ...and we're in the CLI
        // ...then change to this directory
        if options.cli_mode
            && block.block.len() == 1
            && block.block[0].pipelines.len() == 1
            && block.block[0].pipelines[0].list.len() == 1
        {
            if let ClassifiedCommand::Internal(InternalCommand {
                ref name, ref args, ..
            }) = block.block[0].pipelines[0].list[0]
            {
                let internal_name = name;
                let name = args
                    .positional
                    .as_ref()
                    .and_then(|positionals| {
                        positionals.get(0).map(|e| {
                            if let Expression::Literal(Literal::String(ref s)) = e.expr {
                                &s
                            } else {
                                ""
                            }
                        })
                    })
                    .unwrap_or("");

                if internal_name == "run_external"
                    && args
                        .positional
                        .as_ref()
                        .map(|ref v| v.len() == 1)
                        .unwrap_or(true)
                    && args
                        .named
                        .as_ref()
                        .map(NamedArguments::is_empty)
                        .unwrap_or(true)
                    && canonicalize(ctx.shell_manager.path(), name).is_ok()
                    && Path::new(&name).is_dir()
                    && !ctx.host.lock().is_external_cmd(&name)
                {
                    // Here we work differently if we're in Windows because of the expected Windows behavior
                    #[cfg(windows)]
                    {
                        if name.ends_with(':') {
                            // This looks like a drive shortcut. We need to a) switch drives and b) go back to the previous directory we were viewing on that drive
                            // But first, we need to save where we are now
                            let current_path = ctx.shell_manager.path();

                            let split_path: Vec<_> = current_path.split(':').collect();
                            if split_path.len() > 1 {
                                ctx.windows_drives_previous_cwd
                                    .lock()
                                    .insert(split_path[0].to_string(), current_path);
                            }

                            let name = name.to_uppercase();
                            let new_drive: Vec<_> = name.split(':').collect();

                            if let Some(val) =
                                ctx.windows_drives_previous_cwd.lock().get(new_drive[0])
                            {
                                ctx.shell_manager.set_path(val.to_string());
                                return LineResult::Success(line.to_string());
                            } else {
                                ctx.shell_manager
                                    .set_path(format!("{}\\", name.to_string()));
                                return LineResult::Success(line.to_string());
                            }
                        } else {
                            ctx.shell_manager.set_path(name.to_string());
                            return LineResult::Success(line.to_string());
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        ctx.shell_manager.set_path(name.to_string());
                        return LineResult::Success(line.to_string());
                    }
                }
            }
        }

        let input_stream = if options.redirect_stdin {
            let file = futures::io::AllowStdIo::new(std::io::stdin());
            let stream = FramedRead::new(file, MaybeTextCodec::default()).map(|line| {
                if let Ok(line) = line {
                    let primitive = match line {
                        StringOrBinary::String(s) => Primitive::String(s),
                        StringOrBinary::Binary(b) => Primitive::Binary(b.into_iter().collect()),
                    };

                    Ok(Value {
                        value: UntaggedValue::Primitive(primitive),
                        tag: Tag::unknown(),
                    })
                } else {
                    panic!("Internal error: could not read lines of text from stdin")
                }
            });
            stream.to_input_stream()
        } else {
            InputStream::empty()
        };

        trace!("{:#?}", block);

        let result = run_block(&block, ctx, input_stream).await;

        match result {
            Ok(input) => {
                // Running a pipeline gives us back a stream that we can then
                // work through. At the top level, we just want to pull on the
                // values to compute them.
                use futures::stream::TryStreamExt;

                let autoview_cmd = ctx
                    .get_command("autoview")
                    .expect("Could not find autoview command");

                if let Ok(mut output_stream) = ctx
                    .run_command(
                        autoview_cmd,
                        Tag::unknown(),
                        Call::new(
                            Box::new(SpannedExpression::new(
                                Expression::string("autoview".to_string()),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        ),
                        input,
                    )
                    .await
                {
                    loop {
                        match output_stream.try_next().await {
                            Ok(Some(ReturnSuccess::Value(Value {
                                value: UntaggedValue::Error(e),
                                ..
                            }))) => return LineResult::Error(line.to_string(), e),
                            Ok(Some(_item)) => {
                                if ctx.ctrl_c.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => return LineResult::Error(line.to_string(), e),
                        }
                    }
                }

                LineResult::Success(line.to_string())
            }
            Err(err) => LineResult::Error(line.to_string(), err),
        }
    }
}
