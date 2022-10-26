use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

#[derive(Clone)]
pub struct OverlayHide;

impl Command for OverlayHide {
    fn name(&self) -> &str {
        "overlay hide"
    }

    fn usage(&self) -> &str {
        "Hide an active overlay"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay hide")
            .optional("name", SyntaxShape::String, "Overlay to hide")
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
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let overlay_name: Spanned<String> = if let Some(name) = call.opt(engine_state, stack, 0)? {
            name
        } else {
            Spanned {
                item: stack.last_overlay_name()?,
                span: call.head,
            }
        };

        if !stack.is_overlay_active(&overlay_name.item) {
            return Err(ShellError::OverlayNotFoundAtRuntime(
                overlay_name.item,
                overlay_name.span,
            ));
        }

        let keep_env: Option<Vec<Spanned<String>>> =
            call.get_flag(engine_state, stack, "keep-env")?;

        let env_vars_to_keep = if let Some(env_var_names_to_keep) = keep_env {
            let mut env_vars_to_keep = vec![];

            for name in env_var_names_to_keep.into_iter() {
                match stack.get_env_var(engine_state, &name.item) {
                    Some(val) => env_vars_to_keep.push((name.item, val.clone())),
                    None => return Err(ShellError::EnvVarNotFoundAtRuntime(name.item, name.span)),
                }
            }

            env_vars_to_keep
        } else {
            vec![]
        };

        stack.remove_overlay(&overlay_name.item);

        for (name, val) in env_vars_to_keep {
            stack.add_env_var(name, val);
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Keep a custom command after hiding the overlay",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    def bar [] { "bar" }
    overlay hide spam --keep-custom
    bar
    "#,
                result: None,
            },
            Example {
                description: "Hide an overlay created from a file",
                example: r#"'export alias f = "foo"' | save spam.nu
    overlay use spam.nu
    overlay hide spam"#,
                result: None,
            },
            Example {
                description: "Hide the last activated overlay",
                example: r#"module spam { export-env { let-env FOO = "foo" } }
    overlay use spam
    overlay hide"#,
                result: None,
            },
            Example {
                description: "Keep the current working directory when removing an overlay",
                example: r#"overlay new spam
    cd some-dir
    overlay hide --keep-env [ PWD ] spam"#,
                result: None,
            },
        ]
    }
}
