use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_data::config;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::hir::{ClassifiedCommand, Expression, NamedValue, SpannedExpression, Variable};
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
                        None | Some((_, None)) => SyntaxShape::Any,
                        Some((_, Some(shape))) => *shape,
                    },
                )
            })
            .collect()),
        Err(err) => Err(err),
    }
}

type ShapeMap = HashMap<String, (Span, Option<SyntaxShape>)>;

fn check_insert(
    existing: &mut ShapeMap,
    to_add: (String, (Span, Option<SyntaxShape>)),
) -> Result<(), ShellError> {
    match (to_add.1).1 {
        None => match existing.get(&to_add.0) {
            None => {
                existing.insert(to_add.0, to_add.1);
                Ok(())
            }
            Some(_) => Ok(()),
        },
        Some(new) => match existing.insert(to_add.0.clone(), ((to_add.1).0, Some(new))) {
            None => Ok(()),
            Some(exist) => match exist.1 {
                None => Ok(()),
                Some(shape) => match shape {
                    SyntaxShape::Any => Ok(()),
                    shape if shape == new => Ok(()),
                    _ => Err(ShellError::labeled_error(
                        "Type conflict in alias variable use",
                        "creates type conflict",
                        (to_add.1).0,
                    )),
                },
            },
        },
    }
}

fn check_merge(existing: &mut ShapeMap, new: &ShapeMap) -> Result<(), ShellError> {
    for (k, v) in new.iter() {
        check_insert(existing, (k.clone(), *v))?;
    }

    Ok(())
}

fn find_expr_shapes(
    spanned_expr: &SpannedExpression,
    registry: &CommandRegistry,
) -> Result<ShapeMap, ShellError> {
    match &spanned_expr.expr {
        // TODO range will need similar if/when invocations can be parsed within range expression
        Expression::Binary(bin) => find_expr_shapes(&bin.left, registry).and_then(|mut left| {
            find_expr_shapes(&bin.right, registry)
                .and_then(|right| check_merge(&mut left, &right).map(|()| left))
        }),
        Expression::Block(b) => find_block_shapes(&b, registry),
        Expression::Path(path) => match &path.head.expr {
            Expression::Invocation(b) => find_block_shapes(&b, registry),
            Expression::Variable(Variable::Other(var, _)) => {
                let mut result = HashMap::new();
                result.insert(var.to_string(), (spanned_expr.span, None));
                Ok(result)
            }
            _ => Ok(HashMap::new()),
        },
        _ => Ok(HashMap::new()),
    }
}

fn find_block_shapes(block: &Block, registry: &CommandRegistry) -> Result<ShapeMap, ShellError> {
    let apply_shape = |found: ShapeMap, sig_shape: SyntaxShape| -> ShapeMap {
        found
            .iter()
            .map(|(v, sh)| match sh.1 {
                None => (v.clone(), (sh.0, Some(sig_shape))),
                Some(shape) => (v.clone(), (sh.0, Some(shape))),
            })
            .collect()
    };

    let mut arg_shapes = HashMap::new();
    for pipeline in &block.block {
        for classified in &pipeline.list {
            match classified {
                ClassifiedCommand::Expr(spanned_expr) => {
                    let found = find_expr_shapes(&spanned_expr, registry)?;
                    check_merge(&mut arg_shapes, &found)?
                }
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name) {
                        if let Some(positional) = &internal.args.positional {
                            for (i, spanned_expr) in positional.iter().enumerate() {
                                let found = find_expr_shapes(&spanned_expr, registry)?;
                                if i >= signature.positional.len() {
                                    if let Some((sig_shape, _)) = &signature.rest_positional {
                                        check_merge(
                                            &mut arg_shapes,
                                            &apply_shape(found, *sig_shape),
                                        )?;
                                    } else {
                                        unreachable!("should have error'd in parsing");
                                    }
                                } else {
                                    let (pos_type, _) = &signature.positional[i];
                                    match pos_type {
                                        // TODO pass on mandatory/optional?
                                        PositionalType::Mandatory(_, sig_shape)
                                        | PositionalType::Optional(_, sig_shape) => {
                                            check_merge(
                                                &mut arg_shapes,
                                                &apply_shape(found, *sig_shape),
                                            )?;
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(named) = &internal.args.named {
                            for (name, val) in named.iter() {
                                if let NamedValue::Value(_, spanned_expr) = val {
                                    let found = find_expr_shapes(&spanned_expr, registry)?;
                                    match signature.named.get(name) {
                                        None => {
                                            unreachable!("should have error'd in parsing");
                                        }
                                        Some((named_type, _)) => {
                                            if let NamedType::Mandatory(_, sig_shape)
                                            | NamedType::Optional(_, sig_shape) = named_type
                                            {
                                                check_merge(
                                                    &mut arg_shapes,
                                                    &apply_shape(found, *sig_shape),
                                                )?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        unreachable!("registry has lost name it provided");
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
