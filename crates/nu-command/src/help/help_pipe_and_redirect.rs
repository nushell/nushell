use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HelpPipeAndRedirect;

impl Command for HelpPipeAndRedirect {
    fn name(&self) -> &str {
        "help pipe-and-redirect"
    }

    fn description(&self) -> &str {
        "Show help on nushell pipes and redirects."
    }

    fn extra_description(&self) -> &str {
        r#"This command contains basic usage of pipe and redirect symbol, for more detail, check:
https://www.nushell.sh/lang-guide/chapters/pipelines.html"#
    }

    fn signature(&self) -> Signature {
        Signature::build("help pipe-and-redirect")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        Ok([
            HelpExample {
                symbol: "|",
                name: "pipe",
                description: "pipeline stdout of a command to another command",
                example: "^cmd1 | ^cmd2",
            },
            HelpExample {
                symbol: "e>|",
                name: "stderr pipe",
                description: "pipeline stderr of a command to another command",
                example: "^cmd1 e>| ^cmd2",
            },
            HelpExample {
                symbol: "o+e>|",
                name: "stdout and stderr pipe",
                description: "pipeline stdout and stderr of a command to another command",
                example: "^cmd1 o+e>| ^cmd2",
            },
            HelpExample {
                symbol: "o>",
                name: "redirection",
                description: "redirect stdout of a command, overwriting a file",
                example: "^cmd1 o> file.txt",
            },
            HelpExample {
                symbol: "e>",
                name: "stderr redirection",
                description: "redirect stderr of a command, overwriting a file",
                example: "^cmd1 e> file.txt",
            },
            HelpExample {
                symbol: "o+e>",
                name: "stdout and stderr redirection",
                description: "redirect stdout and stderr of a command, overwriting a file",
                example: "^cmd1 o+e> file.txt",
            },
            HelpExample {
                symbol: "o>>",
                name: "redirection append",
                description: "redirect stdout of a command, appending to a file",
                example: "^cmd1 o> file.txt",
            },
            HelpExample {
                symbol: "e>>",
                name: "stderr redirection append",
                description: "redirect stderr of a command, appending to a file",
                example: "^cmd1 e> file.txt",
            },
            HelpExample {
                symbol: "o+e>>",
                name: "stdout and stderr redirection append",
                description: "redirect stdout and stderr of a command, appending to a file",
                example: "^cmd1 o+e> file.txt",
            },
            HelpExample {
                symbol: "o>|",
                name: "",
                description:
                    "UNSUPPORTED, Redirecting stdout to a pipe is the same as normal piping",
                example: "",
            },
        ]
        .into_iter()
        .map(|example| example.into_value(head))
        .collect::<List>()
        .into_value(head)
        .into_pipeline_data())
    }
}

#[derive(IntoValue)]
struct HelpExample {
    symbol: &'static str,
    name: &'static str,
    description: &'static str,
    example: &'static str,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpPipeAndRedirect;
        use crate::test_examples;
        test_examples(HelpPipeAndRedirect {})
    }
}
