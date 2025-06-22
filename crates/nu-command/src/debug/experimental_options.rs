use nu_engine::command_prelude::*;
use nu_experimental::Stability;

#[derive(Clone)]
pub struct DebugExperimentalOptions;

impl Command for DebugExperimentalOptions {
    fn name(&self) -> &str {
        "debug experimental-options"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name())
            .input_output_type(Type::Nothing, Type::Table(Box::from([
                (String::from("identifier"), Type::String),
                (String::from("enabled"), Type::Bool),
                (String::from("stability"), Type::String),
                (String::from("description"), Type::String),
            ])))
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
        Ok(PipelineData::Value(
            Value::list(nu_experimental::ALL.iter().map(|option| {
                Value::record(nu_protocol::record!{
                    "identifier" => Value::string(option.identifier(), call.head),
                    "enabled" => Value::bool(option.get(), call.head),
                    "stability" => Value::string(match option.stability() {
                        Stability::Unstable => "unstable",
                        Stability::StableOptIn => "stable-opt-in",
                        Stability::StableOptOut => "stable-opt-out",
                        Stability::Deprecated => "deprecated"
                    }, call.head),
                    "description" => Value::string(option.description(), call.head),
                }, call.head)
            }).collect(), call.head),
            None
        ))
    }
}
