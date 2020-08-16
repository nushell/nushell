use crate::commands::command::whole_stream_command;
use crate::commands::run_alias::AliasCommand;
use crate::commands::UnevaluatedCallInfo;
use crate::prelude::*;
use log::{log_enabled, trace};
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::hir::{
    Block, ClassifiedCommand, Expression, ExternalRedirection, InternalCommand, NamedValue,
    Variable,
};
use nu_protocol::{
    CommandAction, NamedType, PositionalType, Primitive, ReturnSuccess, Scope, SyntaxShape,
    UntaggedValue, Value,
};

pub(crate) async fn run_internal_command(
    command: InternalCommand,
    context: &mut Context,
    input: InputStream,
    it: &Value,
    vars: &IndexMap<String, Value>,
    env: &IndexMap<String, String>,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::internal", "->");
        trace!(target: "nu::run::internal", "{}", command.name);
    }

    let scope = Scope {
        it: it.clone(),
        vars: vars.clone(),
        env: env.clone(),
    };
    let objects: InputStream = trace_stream!(target: "nu::trace_stream::internal", "input" = input);
    let internal_command = context.expect_command(&command.name);

    if command.name == "autoenv untrust" {
        context.user_recently_used_autoenv_untrust = true;
    }

    let result = {
        context
            .run_command(
                internal_command?,
                Tag::unknown_anchor(command.name_span),
                command.args.clone(),
                &scope,
                objects,
            )
            .await?
    };

    let head = Arc::new(command.args.head.clone());
    //let context = Arc::new(context.clone());
    let context = context.clone();
    let command = Arc::new(command);
    let scope = Arc::new(scope);
    // let scope = scope.clone();

    Ok(InputStream::from_stream(
        result
            .then(move |item| {
                let head = head.clone();
                let command = command.clone();
                let mut context = context.clone();
                let scope = scope.clone();
                async move {
                    match item {
                        Ok(ReturnSuccess::Action(action)) => match action {
                            CommandAction::ChangePath(path) => {
                                context.shell_manager.set_path(path);
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::Exit => std::process::exit(0), // TODO: save history.txt
                            CommandAction::Error(err) => {
                                context.error(err.clone());
                                InputStream::one(UntaggedValue::Error(err).into_untagged_value())
                            }
                            CommandAction::AutoConvert(tagged_contents, extension) => {
                                let contents_tag = tagged_contents.tag.clone();
                                let command_name = format!("from {}", extension);
                                let command = command.clone();
                                if let Some(converter) = context.registry.get_command(&command_name)
                                {
                                    let new_args = RawCommandArgs {
                                        host: context.host.clone(),
                                        ctrl_c: context.ctrl_c.clone(),
                                        current_errors: context.current_errors.clone(),
                                        shell_manager: context.shell_manager.clone(),
                                        call_info: UnevaluatedCallInfo {
                                            args: nu_protocol::hir::Call {
                                                head: (&*head).clone(),
                                                positional: None,
                                                named: None,
                                                span: Span::unknown(),
                                                external_redirection: ExternalRedirection::Stdout,
                                            },
                                            name_tag: Tag::unknown_anchor(command.name_span),
                                            scope: (&*scope).clone(),
                                        },
                                    };
                                    let result = converter
                                        .run(
                                            new_args.with_input(vec![tagged_contents]),
                                            &context.registry,
                                        )
                                        .await;

                                    match result {
                                        Ok(mut result) => {
                                            let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                                                result.drain_vec().await;

                                            let mut output = vec![];
                                            for res in result_vec {
                                                match res {
                                                    Ok(ReturnSuccess::Value(Value {
                                                        value: UntaggedValue::Table(list),
                                                        ..
                                                    })) => {
                                                        for l in list {
                                                            output.push(Ok(l));
                                                        }
                                                    }
                                                    Ok(ReturnSuccess::Value(Value {
                                                        value,
                                                        ..
                                                    })) => {
                                                        output
                                                            .push(Ok(value
                                                                .into_value(contents_tag.clone())));
                                                    }
                                                    Err(e) => output.push(Err(e)),
                                                    _ => {}
                                                }
                                            }

                                            futures::stream::iter(output).to_input_stream()
                                        }
                                        Err(e) => {
                                            context.add_error(e);
                                            InputStream::empty()
                                        }
                                    }
                                } else {
                                    InputStream::one(tagged_contents)
                                }
                            }
                            CommandAction::EnterHelpShell(value) => match value {
                                Value {
                                    value: UntaggedValue::Primitive(Primitive::String(cmd)),
                                    tag,
                                } => {
                                    context.shell_manager.insert_at_current(Box::new(
                                        match HelpShell::for_command(
                                            UntaggedValue::string(cmd).into_value(tag),
                                            &context.registry(),
                                        ) {
                                            Ok(v) => v,
                                            Err(err) => {
                                                return InputStream::one(
                                                    UntaggedValue::Error(err).into_untagged_value(),
                                                )
                                            }
                                        },
                                    ));
                                    InputStream::from_stream(futures::stream::iter(vec![]))
                                }
                                _ => {
                                    context.shell_manager.insert_at_current(Box::new(
                                        match HelpShell::index(&context.registry()) {
                                            Ok(v) => v,
                                            Err(err) => {
                                                return InputStream::one(
                                                    UntaggedValue::Error(err).into_untagged_value(),
                                                )
                                            }
                                        },
                                    ));
                                    InputStream::from_stream(futures::stream::iter(vec![]))
                                }
                            },
                            CommandAction::EnterValueShell(value) => {
                                context
                                    .shell_manager
                                    .insert_at_current(Box::new(ValueShell::new(value)));
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::EnterShell(location) => {
                                context.shell_manager.insert_at_current(Box::new(
                                    match FilesystemShell::with_location(location) {
                                        Ok(v) => v,
                                        Err(err) => {
                                            return InputStream::one(
                                                UntaggedValue::Error(err.into())
                                                    .into_untagged_value(),
                                            )
                                        }
                                    },
                                ));
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::AddAlias(name, args, block) => {
                                let mut arg_shapes: IndexMap<String, SyntaxShape> = args
                                    .iter()
                                    .map(|arg| (arg.clone(), SyntaxShape::Any))
                                    .collect();
                                find_arg_shapes(&block, &context.registry, &mut arg_shapes);

                                context.add_commands(vec![whole_stream_command(
                                    AliasCommand::new(
                                        name,
                                        arg_shapes.into_iter().collect(),
                                        block,
                                    ),
                                )]);
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::PreviousShell => {
                                context.shell_manager.prev();
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::NextShell => {
                                context.shell_manager.next();
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                            CommandAction::LeaveShell => {
                                context.shell_manager.remove_at_current();
                                if context.shell_manager.is_empty() {
                                    std::process::exit(0); // TODO: save history.txt
                                }
                                InputStream::from_stream(futures::stream::iter(vec![]))
                            }
                        },

                        Ok(ReturnSuccess::Value(Value {
                            value: UntaggedValue::Error(err),
                            tag,
                        })) => {
                            context.error(err.clone());
                            InputStream::one(UntaggedValue::Error(err).into_value(tag))
                        }

                        Ok(ReturnSuccess::Value(v)) => InputStream::one(v),

                        Ok(ReturnSuccess::DebugValue(v)) => {
                            let doc = PrettyDebug::pretty_doc(&v);
                            let mut buffer = termcolor::Buffer::ansi();

                            let _ = doc.render_raw(
                                context.with_host(|host| host.width() - 5),
                                &mut nu_source::TermColored::new(&mut buffer),
                            );

                            let value = String::from_utf8_lossy(buffer.as_slice());

                            InputStream::one(UntaggedValue::string(value).into_untagged_value())
                        }

                        Err(err) => {
                            context.error(err.clone());
                            InputStream::one(UntaggedValue::Error(err).into_untagged_value())
                        }
                    }
                }
            })
            .flatten()
            .take_while(|x| futures::future::ready(!x.is_error())),
    ))
}

fn find_expr_shapes<F>(
    expr: &Expression,
    registry: &CommandRegistry,
    arg_shapes: &mut IndexMap<String, SyntaxShape>,
    mut on_found: F,
) where
    F: FnMut(&str, &mut IndexMap<String, SyntaxShape>),
{
    match expr {
        Expression::Block(b) => find_arg_shapes(&b, registry, arg_shapes),
        Expression::Path(path) => {
            match &path.head.expr {
                Expression::Invocation(b) => {
                    // TODO need to kick it up?
                    find_arg_shapes(&b, registry, arg_shapes);
                }
                Expression::Variable(Variable::Other(var, _)) => {
                    if arg_shapes.contains_key(var) {
                        on_found(var, arg_shapes);
                    }
                }
                _ => (),
            }
        }
        _ => (),
    }
}

fn find_arg_shapes(
    block: &Block,
    registry: &CommandRegistry,
    arg_shapes: &mut IndexMap<String, SyntaxShape>,
) {
    for pipeline in &block.block {
        for classified in &pipeline.list {
            match classified {
                ClassifiedCommand::Expr(spanned) => {
                    // TODO binary TODO range?
                    match &spanned.expr {
                        Expression::Block(b) => find_arg_shapes(b, registry, arg_shapes),
                        Expression::Path(path) => {
                            if let Expression::Invocation(b) = &path.head.expr {
                                find_arg_shapes(&b, registry, arg_shapes);
                            }
                        }
                        _ => continue,
                    }
                }
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name) {
                        if let Some(positional) = &internal.args.positional {
                            for (i, spanned) in positional.iter().enumerate() {
                                find_expr_shapes(
                                    &spanned.expr,
                                    registry,
                                    arg_shapes,
                                    |var, shapes| -> () {
                                        if i >= signature.positional.len() {
                                            if let Some((shape, _)) = &signature.rest_positional {
                                                shapes.insert(var.to_string(), shape.clone());
                                            }
                                        } else {
                                            let (pos_type, _) = &signature.positional[i];
                                            match pos_type {
                                                // TODO also use mandatory/optional?
                                                PositionalType::Mandatory(_, shape)
                                                | PositionalType::Optional(_, shape) => {
                                                    shapes.insert(var.to_string(), shape.clone());
                                                }
                                            }
                                        }
                                    },
                                );

                                // match &spanned.expr {
                                //     Expression::Block(b) => {
                                //         find_arg_shapes(b, registry, arg_shapes)
                                //     }
                                //     Expression::Path(path) => {
                                //         match &path.head.expr {
                                //             Expression::Invocation(b) => {
                                //                 // TODO need to kick it up?
                                //                 find_arg_shapes(b, registry, arg_shapes)
                                //             }
                                //             Expression::Variable(Variable::Other(var, _)) => {
                                //                 if arg_shapes.contains_key(var) {
                                //                     if i >= signature.positional.len() {
                                //                         if let Some((shape, _)) =
                                //                             &signature.rest_positional
                                //                         {
                                //                             arg_shapes
                                //                                 .insert(var.clone(), shape.clone());
                                //                         }
                                //                     } else {
                                //                         let (pos_type, _) =
                                //                             &signature.positional[i];
                                //                         match pos_type {
                                //                             // TODO also use mandatory/optional?
                                //                             PositionalType::Mandatory(_, shape)
                                //                             | PositionalType::Optional(_, shape) => {
                                //                                 arg_shapes.insert(
                                //                                     var.clone(),
                                //                                     shape.clone(),
                                //                                 );
                                //                             }
                                //                         }
                                //                     }
                                //                 }
                                //             }
                                //             _ => continue,
                                //         }
                                //     }
                                //     _ => continue,
                                // }
                            }
                        }
                        if let Some(named) = &internal.args.named {
                            for (name, val) in named.iter() {
                                if let NamedValue::Value(_, spanned) = val {
                                    find_expr_shapes(
                                        &spanned.expr,
                                        registry,
                                        arg_shapes,
                                        |var, shapes| {
                                            match signature.named.get(name) {
                                                None => (), // TODO?
                                                Some((named_type, _)) => match named_type {
                                                    NamedType::Mandatory(_, shape)
                                                    | NamedType::Optional(_, shape) => {
                                                        shapes
                                                            .insert(var.to_string(), shape.clone());
                                                    }
                                                    _ => (),
                                                },
                                            }
                                        },
                                    );

                                    // match &spanned.expr {
                                    //     // TODO abstract this out?
                                    //     Expression::Block(b) => {
                                    //         find_arg_shapes(b, registry, arg_shapes)
                                    //     }
                                    //     Expression::Path(path) => {
                                    //         match &path.head.expr {
                                    //             Expression::Invocation(b) => {
                                    //                 // TODO need to kick it up?
                                    //                 find_arg_shapes(b, registry, arg_shapes)
                                    //             }
                                    //             Expression::Variable(Variable::Other(var, _)) => {
                                    //                 if arg_shapes.contains_key(var) {
                                    //                     match signature.named.get(name) {
                                    //                         None => continue, // TODO?
                                    //                         Some((named_type, _)) => {
                                    //                             match named_type {
                                    //                                 NamedType::Mandatory(
                                    //                                     _,
                                    //                                     shape,
                                    //                                 )
                                    //                                 | NamedType::Optional(
                                    //                                     _,
                                    //                                     shape,
                                    //                                 ) => {
                                    //                                     arg_shapes.insert(
                                    //                                         var.clone(),
                                    //                                         shape.clone(),
                                    //                                     );
                                    //                                 }
                                    //                                 _ => continue,
                                    //                             }
                                    //                         }
                                    //                     }
                                    //                 }
                                    //             }
                                    //             _ => continue,
                                    //         }
                                    //     }
                                    //     _ => continue,
                                    // }
                                }
                            }
                        }
                    }
                }
                ClassifiedCommand::Dynamic(_) | ClassifiedCommand::Error(_) => continue,
            }
        }
    }
}
