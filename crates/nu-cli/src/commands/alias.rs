use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_data::config;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::hir::{ClassifiedCommand, Expression, NamedValue, Variable};
use nu_protocol::{
    hir::Block, CommandAction, NamedType, PositionalType, ReturnSuccess, Signature, SyntaxShape,
    UntaggedValue, Value,
};
use nu_source::Tagged;
use std::collections::HashMap;

pub struct Alias;

#[derive(Deserialize)]
pub struct AliasArgs {
    pub name: Tagged<String>,
    pub args: Vec<Value>,
    pub block: Block,
    pub save: Option<bool>,
}

#[async_trait]
impl WholeStreamCommand for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn signature(&self) -> Signature {
        Signature::build("alias")
            .required("name", SyntaxShape::String, "the name of the alias")
            .required("args", SyntaxShape::Table, "the arguments to the alias")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run as the body of the alias",
            )
            .switch("save", "save the alias to your config", Some('s'))
    }

    fn usage(&self) -> &str {
        "Define a shortcut for another command."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        alias(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "An alias without parameters",
                example: "alias say-hi [] { echo 'Hello!' }",
                result: None,
            },
            Example {
                description: "An alias with a single parameter",
                example: "alias l [x] { ls $x }",
                result: None,
            },
        ]
    }
}

pub async fn alias(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let mut raw_input = args.raw_input.clone();
    let (
        AliasArgs {
            name,
            args: list,
            block,
            save,
        },
        _ctx,
    ) = args.process(&registry).await?;
    let mut processed_args: Vec<String> = vec![];

    if let Some(true) = save {
        let mut result = nu_data::config::read(name.clone().tag, &None)?;

        // process the alias to remove the --save flag
        let left_brace = raw_input.find('{').unwrap_or(0);
        let right_brace = raw_input.rfind('}').unwrap_or_else(|| raw_input.len());
        let left = raw_input[..left_brace]
            .replace("--save", "")
            .replace("-s", "");
        let right = raw_input[right_brace..]
            .replace("--save", "")
            .replace("-s", "");
        raw_input = format!("{}{}{}", left, &raw_input[left_brace..right_brace], right);

        // create a value from raw_input alias
        let alias: Value = raw_input.trim().to_string().into();
        let alias_start = raw_input.find('[').unwrap_or(0); // used to check if the same alias already exists

        // add to startup if alias doesn't exist and replce if it does
        match result.get_mut("startup") {
            Some(startup) => {
                if let UntaggedValue::Table(ref mut commands) = startup.value {
                    if let Some(command) = commands.iter_mut().find(|command| {
                        let cmd_str = command.as_string().unwrap_or_default();
                        cmd_str.starts_with(&raw_input[..alias_start])
                    }) {
                        *command = alias;
                    } else {
                        commands.push(alias);
                    }
                }
            }
            None => {
                let table = UntaggedValue::table(&[alias]);
                result.insert("startup".to_string(), table.into_value(Tag::default()));
            }
        }
        config::write(&result, &None)?;
    }

    for item in list.iter() {
        if let Ok(string) = item.as_string() {
            processed_args.push(format!("${}", string));
        } else {
            return Err(ShellError::labeled_error(
                "Expected a string",
                "expected a string",
                item.tag(),
            ));
        }
    }

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddAlias(
            name.to_string(),
            to_arg_shapes(processed_args, &block, &registry)?,
            block,
        ),
    )))
}

// match find_block_shapes(&block, &context.registry) {
//     Ok(found) => {
//         let arg_shapes: IndexMap<String, SyntaxShape> = args
//             .iter()
//             .map(|arg| {
//                 (
//                     arg.clone(),
//                     match found.get(arg) {
//                         None | Some(None) => SyntaxShape::Any,
//                         Some(Some(shape)) => *shape,
//                     },
//                 )
//             })
//             .collect();
//
//         context.add_commands(vec![whole_stream_command(
//             AliasCommand::new(
//                 name,
//                 arg_shapes.into_iter().collect(),
//                 block,
//             ),
//         )]);
//         InputStream::from_stream(futures::stream::iter(vec![]))
//     }
//     Err(err) => InputStream::one(
//         UntaggedValue::Error(err).into_untagged_value(),
//     ),
// }

fn to_arg_shapes(
    args: Vec<String>,
    block: &Block,
    registry: &CommandRegistry,
) -> Result<Vec<(String, SyntaxShape)>, ShellError> {
    match find_block_shapes(block, registry) {
        Ok(found) => Ok(args
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
            .collect()),
        Err(err) => Err(err),
    }
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
    existing: &mut HashMap<String, Option<SyntaxShape>>,
    new: &HashMap<String, Option<SyntaxShape>>,
) -> Result<(), ShellError> {
    for (k, v) in new.iter() {
        check_insert(existing, (k.clone(), v.clone()))?;
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

    Ok(arg_shapes)
}

#[cfg(test)]
mod tests {
    use super::Alias;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Alias {})
    }
}
