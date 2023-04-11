use super::{list_shells, switch_shell, SwitchTo};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct GotoShell;

impl Command for GotoShell {
    fn name(&self) -> &str {
        "g"
    }

    fn signature(&self) -> Signature {
        Signature::build("g")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Nothing, Type::Table(vec![])),
            ])
            .optional(
                "shell_number",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::String]),
                "shell number to change to",
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Switch to a given shell, or list all shells if no given shell number."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let new_shell: Option<Value> = call.opt(engine_state, stack, 0)?;

        match new_shell {
            Some(shell_span) => match &shell_span {
                Value::String { val, span } => {
                    if val == "-" {
                        switch_shell(engine_state, stack, call, *span, SwitchTo::Last)
                    } else {
                        Err(ShellError::TypeMismatch {
                            err_message: "int or '-'".into(),
                            span: *span,
                        })
                    }
                }
                Value::Int { val, span } => switch_shell(
                    engine_state,
                    stack,
                    call,
                    *span,
                    SwitchTo::Nth(*val as usize),
                ),
                _ => Err(ShellError::TypeMismatch {
                    err_message: "int or '-'".into(),
                    span: call.head,
                }),
            },
            None => list_shells(engine_state, stack, call.head),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Lists all open shells",
                example: r#"g"#,
                result: None,
            },
            Example {
                description: "Make two directories and enter new shells for them, use `g` to jump to the specific shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar; g 1"#,
                result: None,
            },
            Example {
                description: "Use `shells` to show all the opened shells and run `g 2` to jump to the third one",
                example: r#"shells; g 2"#,
                result: None,
            },
            Example {
                description: "Make two directories and enter new shells for them, use `g -` to jump to the last used shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar; g -"#,
                result: None,
            },
        ]
    }
}
