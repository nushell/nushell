use crate::command_registry::CommandRegistry;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use crate::types::deduction::{VarDeclaration, VarSyntaxShapeDeductor};
use deduction_to_signature::DeductionToSignature;
use log::trace;
use nu_data::config;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Alias;

#[derive(Deserialize)]
pub struct AliasArgs {
    pub name: Tagged<String>,
    pub args: Vec<Value>,
    pub block: Block,
    pub infer: Option<bool>,
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
            .switch("infer", "infer argument types (experimental)", Some('i'))
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
            infer,
            save,
        },
        _ctx,
    ) = args.process(&registry).await?;

    if let Some(true) = save {
        let mut result = nu_data::config::read(name.clone().tag, &None)?;

        // process the alias to remove the --save flag
        let left_brace = raw_input.find('{').unwrap_or(0);
        let right_brace = raw_input.rfind('}').unwrap_or_else(|| raw_input.len());
        let left = raw_input[..left_brace]
            .replace("--save", "") // TODO using regex (or reconstruct string from AST?)
            .replace("-si", "-i")
            .replace("-s ", "")
            .replace("-is", "-i");
        let right = raw_input[right_brace..]
            .replace("--save", "")
            .replace("-si", "-i")
            .replace("-s ", "")
            .replace("-is", "-i");
        raw_input = format!("{}{}{}", left, &raw_input[left_brace..right_brace], right);

        // create a value from raw_input alias
        let alias: Value = raw_input.trim().to_string().into();
        let alias_start = raw_input.find('[').unwrap_or(0); // used to check if the same alias already exists

        // add to startup if alias doesn't exist and replace if it does
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

    let mut processed_args: Vec<VarDeclaration> = vec![];
    for (_, item) in list.iter().enumerate() {
        match item.as_string() {
            Ok(var_name) => {
                let dollar_var_name = format!("${}", var_name);
                processed_args.push(VarDeclaration {
                    name: dollar_var_name,
                    // type_decl: None,
                    span: item.tag.span,
                });
            }
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "Expected a string",
                    "expected a string",
                    item.tag(),
                ));
            }
        }
    }
    trace!("Found vars: {:?}", processed_args);

    let inferred_shapes = {
        if let Some(true) = infer {
            VarSyntaxShapeDeductor::infer_vars(&processed_args, &block, &registry)?
        } else {
            processed_args.into_iter().map(|arg| (arg, None)).collect()
        }
    };
    let signature = DeductionToSignature::get(&name.item, &inferred_shapes);
    trace!("Inferred signature: {:?}", signature);

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddAlias(Box::new(signature), block),
    )))
}

#[cfg(test)]
mod tests {
    use super::Alias;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(Alias {})?)
    }
}

mod deduction_to_signature {
    //For now this logic is relativly simple.
    //For each var, one mandatory positional is added.
    //As soon as more support for optional positional arguments is arrived,
    //this logic might be a little bit more tricky.
    use crate::types::deduction::{Deduction, VarDeclaration};
    use nu_protocol::{PositionalType, Signature, SyntaxShape};

    pub struct DeductionToSignature {}
    impl DeductionToSignature {
        pub fn get(
            cmd_name: &str,
            deductions: &[(VarDeclaration, Option<Deduction>)],
        ) -> Signature {
            let mut signature = Signature::build(cmd_name);
            for (decl, deduction) in deductions {
                match deduction {
                    None => signature.positional.push((
                        PositionalType::optional(&decl.name, SyntaxShape::Any),
                        decl.name.clone(),
                    )),
                    Some(deduction) => match deduction {
                        Deduction::VarShapeDeduction(normal_var_deduction) => {
                            signature.positional.push((
                                PositionalType::optional(
                                    &decl.name,
                                    normal_var_deduction[0].deduction,
                                ),
                                decl.name.clone(),
                            ))
                        }
                    },
                }
            }
            signature
        }
    }
}
