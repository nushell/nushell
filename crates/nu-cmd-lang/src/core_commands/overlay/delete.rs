use super::OverlayHide;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct OverlayDelete;

impl Command for OverlayDelete {
    fn name(&self) -> &str {
        "overlay delete"
    }

    fn usage(&self) -> &str {
        "delete an active overlay."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay delete")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .optional("name", SyntaxShape::String, "Overlay to delete")
            .switch(
                "keep-custom",
                "Keep all newly added commands and aliases in the next activated overlay",
                Some('k'),
            )
            .named(
                "keep-env",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "List of environment variables to keep in the next activated overlay",
                Some('e'),
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // only different to `overlay hide` in parse stage.
        let hide_cmd = OverlayHide;
        hide_cmd.run(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Keep a custom command after hiding the overlay",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    def bar [] { "bar" }
    overlay delete spam --keep-custom
    bar
    "#,
                result: None,
            },
            Example {
                description: "Delete an overlay created from a file",
                example: r#"'export alias f = "foo"' | save spam.nu
    overlay use spam.nu
    overlay delete spam"#,
                result: None,
            },
            Example {
                description: "Delete the last activated overlay",
                example: r#"module spam { export-env { let-env FOO = "foo" } }
    overlay use spam
    overlay delete"#,
                result: None,
            },
            Example {
                description: "Keep the current working directory when deleting an overlay",
                example: r#"overlay new spam
    cd some-dir
    overlay delete --keep-env [ PWD ] spam"#,
                result: None,
            },
        ]
    }
}
