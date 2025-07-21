use nu_engine::command_prelude::*;
use nu_experimental::Status;

#[derive(Clone)]
pub struct DebugExperimentalOptions;

impl Command for DebugExperimentalOptions {
    fn name(&self) -> &str {
        "debug experimental-options"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name())
            .input_output_type(
                Type::Nothing,
                Type::Table(Box::from([
                    (String::from("identifier"), Type::String),
                    (String::from("enabled"), Type::Bool),
                    (String::from("status"), Type::String),
                    (String::from("description"), Type::String),
                ])),
            )
            .add_help()
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Show all experimental options."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            Value::list(
                nu_experimental::ALL
                    .iter()
                    .map(|option| {
                        Value::record(
                            nu_protocol::record! {
                                "identifier" => Value::string(option.identifier(), call.head),
                                "enabled" => Value::bool(option.get(), call.head),
                                "status" => Value::string(match option.status() {
                                    Status::OptIn => "opt-in",
                                    Status::OptOut => "opt-out",
                                    Status::DeprecatedDiscard => "deprecated-discard",
                                    Status::DeprecatedDefault => "deprecated-default"
                                }, call.head),
                                "description" => Value::string(option.description(), call.head),
                            },
                            call.head,
                        )
                    })
                    .collect(),
                call.head,
            ),
            None,
        ))
    }
}
