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
use std::collections::HashMap;

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
                                match find_block_shapes(&block, &context.registry) {
                                    Ok(found) => {
                                        let arg_shapes: IndexMap<String, SyntaxShape> = args
                                            .iter()
                                            .map(|arg| {
                                                (
                                                    arg.clone(),
                                                    match found.get(arg) {
                                                        None | Some(None) => SyntaxShape::Any,
                                                        Some(Some(shape)) => *shape,
                                                    },
                                                )
                                            })
                                            .collect();

                                        context.add_commands(vec![whole_stream_command(
                                            AliasCommand::new(
                                                name,
                                                arg_shapes.into_iter().collect(),
                                                block,
                                            ),
                                        )]);
                                        InputStream::from_stream(futures::stream::iter(vec![]))
                                    }
                                    Err(err) => InputStream::one(
                                        UntaggedValue::Error(err).into_untagged_value(),
                                    ),
                                }
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

fn check_insert(
    existing: &mut HashMap<String, Option<SyntaxShape>>,
    to_add: (String, Option<SyntaxShape>),
) -> Result<(), ShellError> {
    match to_add.1 {
        None => Ok(()),
        Some(new) => match existing.insert(to_add.0.to_string(), Some(new.clone())) {
            None => Ok(()),
            Some(exist) => match exist {
                None => Ok(()),
                Some(shape) => {
                    match shape {
                        SyntaxShape::Any => Ok(()),
                        shape if shape == new => Ok(()),
                        _ => {
                            return Err(ShellError::untagged_runtime_error(String::from(
                                "alias syntax shape mismatch",
                            ))); // TODO .spanned(err_span),
                                 // ))
                        }
                    }
                }
            },
        },
    }
}

fn check_merge(
    // TODO maybe get rid of?
    existing: &mut HashMap<String, Option<SyntaxShape>>,
    new: &HashMap<String, Option<SyntaxShape>>,
) -> Result<(), ShellError> {
    for (k, v) in new.iter() {
        check_insert(existing, (k.clone(), v.clone()))?; // TODO cloning?
    }

    Ok(())
}

fn find_expr_shapes(
    expr: &Expression,
    registry: &CommandRegistry,
) -> Result<HashMap<String, Option<SyntaxShape>>, ShellError> {
    match expr {
        // TODO range?
        // TODO does Invocation ever show up here?
        Expression::Binary(bin) => {
            find_expr_shapes(&bin.left.expr, registry).and_then(|mut left| {
                find_expr_shapes(&bin.right.expr, registry)
                    .and_then(|right| check_merge(&mut left, &right).map(|()| left))
            })
        }
        Expression::Block(b) => find_block_shapes(&b, registry),
        Expression::Path(path) => match &path.head.expr {
            Expression::Invocation(b) => find_block_shapes(&b, registry),
            Expression::Variable(Variable::Other(var, _)) => {
                let mut result = HashMap::new();
                result.insert(var.to_string(), None);
                Ok(result)
            }
            _ => Ok(HashMap::new()),
        },
        _ => Ok(HashMap::new()),
    }
}

fn find_block_shapes(
    block: &Block,
    registry: &CommandRegistry,
) -> Result<HashMap<String, Option<SyntaxShape>>, ShellError> {
    let mut arg_shapes = HashMap::new();
    for pipeline in &block.block {
        // println!("{:#?}", pipeline);
        for classified in &pipeline.list {
            match classified {
                ClassifiedCommand::Expr(spanned) => {
                    let found = find_expr_shapes(&spanned.expr, registry)?;
                    check_merge(&mut arg_shapes, &found)?
                }
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name) {
                        if let Some(positional) = &internal.args.positional {
                            for (i, spanned) in positional.iter().enumerate() {
                                // TODO use spanned
                                let found = find_expr_shapes(&spanned.expr, registry)?;
                                if i >= signature.positional.len() {
                                    if let Some((sig_shape, _)) = &signature.rest_positional {
                                        for (v, sh) in found.iter() {
                                            match sh {
                                                None => check_insert(
                                                    &mut arg_shapes,
                                                    (v.to_string(), Some(*sig_shape)),
                                                ),
                                                Some(shape) => check_insert(
                                                    // TODO is this a problem?
                                                    &mut arg_shapes,
                                                    (v.to_string(), Some(*shape)),
                                                ),
                                            };
                                        }
                                    } else {
                                        return Err(ShellError::unimplemented(
                                            "TODO too many positionals",
                                        ));
                                    }
                                } else {
                                    let (pos_type, _) = &signature.positional[i];
                                    match pos_type {
                                        // TODO also use mandatory/optional?
                                        PositionalType::Mandatory(_, sig_shape)
                                        | PositionalType::Optional(_, sig_shape) => {
                                            found
                                                .iter()
                                                .map(|(v, sh)| {
                                                    match sh {
                                                        None => check_insert(
                                                            &mut arg_shapes,
                                                            (v.to_string(), Some(*sig_shape)),
                                                        ),
                                                        Some(shape) => check_insert(
                                                            // TODO is this a problem?
                                                            &mut arg_shapes,
                                                            (v.to_string(), Some(*shape)),
                                                        ),
                                                    }
                                                })
                                                .collect::<Result<Vec<_>, _>>()?;
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(named) = &internal.args.named {
                            for (name, val) in named.iter() {
                                if let NamedValue::Value(_, spanned) = val {
                                    let found = find_expr_shapes(&spanned.expr, registry)?;
                                    match signature.named.get(name) {
                                        None => {
                                            return Err(ShellError::unimplemented(
                                                "TODO invalid named arg, make a spanned error",
                                            ))
                                        }
                                        Some((named_type, _)) => {
                                            if let NamedType::Mandatory(_, sig_shape)
                                            | NamedType::Optional(_, sig_shape) = named_type
                                            {
                                                for (v, sh) in found.iter() {
                                                    match sh {
                                                        None => check_insert(
                                                            &mut arg_shapes,
                                                            (v.to_string(), Some(*sig_shape)),
                                                        ),
                                                        Some(shape) => check_insert(
                                                            // TODO is this a problem?
                                                            &mut arg_shapes,
                                                            (v.to_string(), Some(*shape)),
                                                        ),
                                                    };
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // } else {  TODO unreachable (will always be run_external)
                        //     println!("aint fucking here");
                        //     return Err(ShellError::unimplemented("TODO same as command not found"));
                    }
                }
                ClassifiedCommand::Dynamic(_) | ClassifiedCommand::Error(_) => (),
            }
        }
    }

    // println!("still gonna return OK");
    Ok(arg_shapes)
    // Err(ShellError::unimplemented("TODO"))
}

// fn find_expr_shapes<F>(
//     expr: &Expression,
//     registry: &CommandRegistry,
//     arg_shapes: &mut IndexMap<String, SyntaxShape>,
//     mut on_found: F,
// ) -> Result<(), ShellError>
// where
//     F: FnMut(&str, &mut IndexMap<String, SyntaxShape>) -> Result<(), ShellError>,
// {
//     match expr {
//         // TODO range?
//         // TODO does Invocation ever show up here?
//         Expression::Binary(bin) => {
//             find_expr_shapes(
//                 &bin.left.expr,
//                 registry,
//                 arg_shapes,
//                 |_var, _shapes| Ok(()), // TODO kick up??
//             )
//             .and_then(|()| {
//                 find_expr_shapes(
//                     &bin.right.expr,
//                     registry,
//                     arg_shapes,
//                     |_var, _shapes| Ok(()),
//                 )
//             })
//         }
//         Expression::Block(b) => find_block_shapes(&b, registry, arg_shapes),
//         Expression::Path(path) => match &path.head.expr {
//             Expression::Invocation(b) => find_block_shapes(&b, registry, arg_shapes),
//             Expression::Variable(Variable::Other(var, _)) => {
//                 if arg_shapes.contains_key(var) {
//                     on_found(var, arg_shapes)
//                 } else {
//                     Ok(())
//                 }
//             }
//             _ => Ok(()),
//         },
//         _ => Ok(()),
//     }
// }
//
// fn find_block_shapes(
//     block: &Block,
//     registry: &CommandRegistry,
//     arg_shapes: &mut IndexMap<String, SyntaxShape>,
// ) -> Result<(), ShellError> {
//     for pipeline in &block.block {
//         for classified in &pipeline.list {
//             match classified {
//                 ClassifiedCommand::Expr(spanned) => {
//                     find_expr_shapes(&spanned.expr, registry, arg_shapes, |_var, _shapes| Ok(()))?
//                 }
//                 ClassifiedCommand::Internal(internal) => {
//                     if let Some(signature) = registry.get(&internal.name) {
//                         let insert_check = |name: &str,
//                                             shp_map: &mut IndexMap<String, SyntaxShape>,
//                                             shp: SyntaxShape,
//                                             err_span: Span|
//                          -> Result<(), ShellError> {
//                             match shp_map
//                                 .insert(name.to_string(), shp.clone())
//                                 .expect("should have been preloaded in map")
//                             {
//                                 SyntaxShape::Any => Ok(()),
//                                 other if other == shp => Ok(()),
//                                 _ => Err(ShellError::syntax_error(
//                                     String::from("alias syntax shape mismatch").spanned(err_span),
//                                 )),
//                             }
//                         };
//                         if let Some(positional) = &internal.args.positional {
//                             for (i, spanned) in positional.iter().enumerate() {
//                                 find_expr_shapes(
//                                     &spanned.expr,
//                                     registry,
//                                     arg_shapes,
//                                     |var, arg_shapes| {
//                                         if i >= signature.positional.len() {
//                                             if let Some((shape, _)) = &signature.rest_positional {
//                                                 insert_check(
//                                                     var,
//                                                     arg_shapes,
//                                                     shape.clone(),
//                                                     spanned.span,
//                                                 )
//                                             } else {
//                                                 Ok(())
//                                             }
//                                         } else {
//                                             let (pos_type, _) = &signature.positional[i];
//                                             match pos_type {
//                                                 // TODO also use mandatory/optional?
//                                                 PositionalType::Mandatory(_, shape)
//                                                 | PositionalType::Optional(_, shape) => {
//                                                     insert_check(
//                                                         var,
//                                                         arg_shapes,
//                                                         shape.clone(),
//                                                         spanned.span,
//                                                     )
//                                                 }
//                                             }
//                                         }
//                                     },
//                                 )?;
//                             }
//                         }
//                         if let Some(named) = &internal.args.named {
//                             for (name, val) in named.iter() {
//                                 if let NamedValue::Value(_, spanned) = val {
//                                     find_expr_shapes(
//                                         &spanned.expr,
//                                         registry,
//                                         arg_shapes,
//                                         |var, arg_shapes| {
//                                             match signature.named.get(name) {
//                                                 None => Ok(()), // TODO?
//                                                 Some((named_type, _)) => match named_type {
//                                                     NamedType::Mandatory(_, shape)
//                                                     | NamedType::Optional(_, shape) => {
//                                                         insert_check(
//                                                             var,
//                                                             arg_shapes,
//                                                             shape.clone(),
//                                                             spanned.span,
//                                                         )
//                                                     }
//                                                     _ => Ok(()),
//                                                 },
//                                             }
//                                         },
//                                     )?;
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 ClassifiedCommand::Dynamic(_) | ClassifiedCommand::Error(_) => continue,
//             }
//         }
//     }
//
//     Ok(())
// }
