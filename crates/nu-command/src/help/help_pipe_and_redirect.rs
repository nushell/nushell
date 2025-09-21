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
        let examples = vec![
            HelpExamples::new(
                "|",
                "pipe",
                "pipeline stdout of a command to another command",
                "^cmd1 | ^cmd2",
            ),
            HelpExamples::new(
                "e>|",
                "stderr pipe",
                "pipeline stderr of a command to another command",
                "^cmd1 e>| ^cmd2",
            ),
            HelpExamples::new(
                "o+e>|",
                "stdout and stderr pipe",
                "pipeline stdout and stderr of a command to another command",
                "^cmd1 o+e>| ^cmd2",
            ),
            HelpExamples::new(
                "o>",
                "redirection",
                "redirect stdout of a command, overwriting a file",
                "^cmd1 o> file.txt",
            ),
            HelpExamples::new(
                "e>",
                "stderr redirection",
                "redirect stderr of a command, overwriting a file",
                "^cmd1 e> file.txt",
            ),
            HelpExamples::new(
                "o+e>",
                "stdout and stderr redirection",
                "redirect stdout and stderr of a command, overwriting a file",
                "^cmd1 o+e> file.txt",
            ),
            HelpExamples::new(
                "o>>",
                "redirection append",
                "redirect stdout of a command, appending to a file",
                "^cmd1 o>> file.txt",
            ),
            HelpExamples::new(
                "e>>",
                "stderr redirection append",
                "redirect stderr of a command, appending to a file",
                "^cmd1 e>> file.txt",
            ),
            HelpExamples::new(
                "o+e>>",
                "stdout and stderr redirection append",
                "redirect stdout and stderr of a command, appending to a file",
                "^cmd1 o+e>> file.txt",
            ),
            HelpExamples::new(
                "o>|",
                "",
                "UNSUPPORTED, Redirecting stdout to a pipe is the same as normal piping",
                "",
            ),
        ];
        let examples: Vec<Value> = examples
            .into_iter()
            .map(|x| x.into_val_record(head))
            .collect();
        Ok(Value::list(examples, head).into_pipeline_data())
    }
}

struct HelpExamples {
    symbol: String,
    name: String,
    description: String,
    example: String,
}

impl HelpExamples {
    fn new(symbol: &str, name: &str, description: &str, example: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            example: example.to_string(),
        }
    }

    fn into_val_record(self, span: Span) -> Value {
        Value::record(
            record! {
                "symbol" => Value::string(self.symbol, span),
                "name" => Value::string(self.name, span),
                "description" => Value::string(self.description, span),
                "example" => Value::string(self.example, span),
            },
            span,
        )
    }
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
