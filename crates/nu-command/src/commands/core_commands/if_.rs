use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue,
};
use nu_stream::OutputStream;

pub struct If;

impl WholeStreamCommand for If {
    fn name(&self) -> &str {
        "if"
    }

    fn signature(&self) -> Signature {
        Signature::build("if")
            .required(
                "condition",
                SyntaxShape::MathExpression,
                "the condition that must match",
            )
            .required(
                "then_case",
                SyntaxShape::Block,
                "block to run if condition is true",
            )
            .required(
                "else_case",
                SyntaxShape::Block,
                "block to run if condition is false",
            )
    }

    fn usage(&self) -> &str {
        "Run blocks if a condition is true or false."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        if_command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run a block if a condition is true",
                example: "let x = 10; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("greater than 5").into()]),
            },
            Example {
                description: "Run a block if a condition is false",
                example: "let x = 1; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("less than or equal to 5").into()]),
            },
        ]
    }
}
fn if_command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let external_redirection = args.call_info.args.external_redirection;
    let context = Arc::new(args.context.clone());

    let condition: CapturedBlock = args.req(0)?;
    let then_case: CapturedBlock = args.req(1)?;
    let else_case: CapturedBlock = args.req(2)?;
    let input = args.input;

    let cond = {
        if condition.block.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a condition",
                "expected a condition",
                tag,
            ));
        }
        match condition.block.block[0].pipelines.get(0) {
            Some(item) => match item.list.get(0) {
                Some(ClassifiedCommand::Expr(expr)) => expr,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Expected a condition",
                        "expected a condition",
                        tag,
                    ));
                }
            },
            None => {
                return Err(ShellError::labeled_error(
                    "Expected a condition",
                    "expected a condition",
                    tag,
                ));
            }
        }
    };

    context.scope.enter_scope();
    context.scope.add_vars(&condition.captured.entries);

    //FIXME: should we use the scope that's brought in as well?
    let condition = evaluate_baseline_expr(cond, &context);
    let result = match condition {
        Ok(condition) => match condition.as_bool() {
            Ok(b) => {
                if b {
                    run_block(&then_case.block, &context, input, external_redirection)
                } else {
                    run_block(&else_case.block, &context, input, external_redirection)
                }
            }
            Err(e) => Ok(OutputStream::from_stream(
                vec![UntaggedValue::Error(e).into_untagged_value()].into_iter(),
            )),
        },
        Err(e) => Ok(OutputStream::from_stream(
            vec![UntaggedValue::Error(e).into_untagged_value()].into_iter(),
        )),
    };
    context.scope.exit_scope();
    result
}

#[cfg(test)]
mod tests {
    use super::If;
    use super::ShellError;
    use nu_test_support::nu;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(If {})
    }

    #[test]
    fn if_doesnt_leak_on_error() {
        let actual = nu!(
            ".",
            r#"
                def test-leak [] {
                    let var = "hello"
                    if 0 == "" {echo ok} {echo not}
                }
                test-leak
                echo $var
            "#
        );

        assert!(actual.err.contains("unknown variable"));
    }
}
