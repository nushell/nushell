use std::convert::Infallible;

use nu_engine::{eval_call, eval_external, CallExt};
use nu_parser::{lite_parse, parse_builtin_commands, Token};
use nu_protocol::ast::{Argument, Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Apply;

impl Command for Apply {
    fn name(&self) -> &str {
        "apply"
    }

    fn usage(&self) -> &str {
        "Apply a list of arguments to a command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .required(
                "command",
                SyntaxShape::Any,
                "the command to apply arguments to",
            )
            .rest(
                "args",
                SyntaxShape::Any,
                "arguments to apply to the command",
            )
            .category(Category::Misc)
            .input_output_types(vec![(Type::Any, Type::Any)])
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "apply \"git\" [\"add\" \"src/main.rs\"]",
            description: "Run the git command with the arguments provided",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        fn value_as_spanned(value: Value) -> Result<Spanned<String>, ShellError> {
            let span = value.span()?;

            value
                .as_string()
                .map(|item| Spanned { item, span })
                .map_err(|_| ShellError::ExternalCommand {
                    label: format!("Cannot convert {} to a string", value.get_type()),
                    help: "All arguments to an external command need to be string-compatible"
                        .into(),
                    span,
                })
        }
        fn mkerr(command: Spanned<String>) -> Result<PipelineData, ShellError> {
            return Err(ShellError::GenericError(
                "Must parse as a single command".to_string(),
                "must parse as a single command".to_string(),
                Some(command.span),
                None,
                vec![],
            ));
        }

        let command: Spanned<String> = call.req(engine_state, stack, 0)?;
        let args: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let mut spanned_args = vec![];
        let args_expr: Vec<Expression> = call.positional_iter().skip(1).cloned().collect();
        let mut arg_keep_raw = vec![];
        for (one_arg, one_arg_expr) in args.into_iter().zip(args_expr) {
            match one_arg {
                Value::List { vals, .. } => {
                    // turn all the strings in the array into params.
                    // Example: one_arg may be something like ["ls" "-a"]
                    // convert it to "ls" "-a"
                    for v in vals {
                        spanned_args.push(value_as_spanned(v)?);
                        // for arguments in list, it's always treated as a whole arguments
                        arg_keep_raw.push(true);
                    }
                }
                val => {
                    spanned_args.push(value_as_spanned(val)?);
                    match one_arg_expr.expr {
                        // refer to `parse_dollar_expr` function
                        // the expression type of $variable_name, $"($variable_name)"
                        // will be Expr::StringInterpolation, Expr::FullCellPath
                        Expr::StringInterpolation(_) | Expr::FullCellPath(_) => {
                            arg_keep_raw.push(true)
                        }
                        _ => arg_keep_raw.push(false),
                    }
                }
            }
        }
        let mut tokens = vec![Token::new(nu_parser::TokenContents::Item, command.span)];
        let spanned_args = spanned_args
            .iter()
            .map(|arg| Token::new(nu_parser::TokenContents::Item, arg.span));
        tokens.extend(spanned_args);
        let (lite_block, err) = lite_parse(&tokens);
        if lite_block.block.len() != 1 {
            return Err(ShellError::GenericError(
                "Expected a single command".to_string(),
                "expected a single command".to_string(),
                Some(command.span),
                None,
                vec![],
            ));
        }
        if lite_block.block[0].commands.len() != 1 {
            return Err(ShellError::GenericError(
                "Expected a single command".to_string(),
                "expected a single command".to_string(),
                Some(command.span),
                None,
                vec![],
            ));
        }
        let mut working_set = StateWorkingSet::new(engine_state);
        let pipeline = match &lite_block.block[0].commands[0] {
            nu_parser::LiteElement::Command(span, lite_command) => {
                parse_builtin_commands(&mut working_set, &lite_command, false)
            }
            nu_parser::LiteElement::Redirection(_, _, _) => {
                return Err(ShellError::GenericError(
                    "Redirection is not supported".to_string(),
                    "Redirection is not supported".to_string(),
                    Some(command.span),
                    None,
                    vec![],
                ))
            }
            nu_parser::LiteElement::SeparateRedirection { out: _, err: _ } => {
                return Err(ShellError::GenericError(
                    "Redirection is not supported".to_string(),
                    "Redirection is not supported".to_string(),
                    Some(command.span),
                    None,
                    vec![],
                ))
            }
        };

        if pipeline.elements.len() != 1 {
            return Err(ShellError::GenericError(
                "Expected a single command".to_string(),
                "expected a single command".to_string(),
                Some(command.span),
                None,
                vec![],
            ));
        }
        let element = &pipeline.elements[0];
        let expr = match element {
            nu_protocol::ast::PipelineElement::Expression(span, expr) => expr,
            nu_protocol::ast::PipelineElement::Redirection(_, _, _) => return mkerr(command),
            nu_protocol::ast::PipelineElement::SeparateRedirection { out, err } => {
                return mkerr(command)
            }
            nu_protocol::ast::PipelineElement::And(_, _) => return mkerr(command),
            nu_protocol::ast::PipelineElement::Or(_, _) => return mkerr(command),
        };
        match &expr.expr {
            Expr::Call(call) => Ok(eval_call(engine_state, stack, call, input)?),
            Expr::ExternalCall(head, args, is_subexpression) => {
                let span = head.span;
                // FIXME: protect this collect with ctrl-c
                Ok(eval_external(
                    engine_state,
                    stack,
                    head,
                    args,
                    input,
                    false,
                    false,
                    *is_subexpression,
                )?)
            }
            _ => return mkerr(command),
        }
    }
}
