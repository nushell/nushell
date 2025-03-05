use nu_engine::command_prelude::*;
use uuid::{Timestamp, Uuid};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random uuid1"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid1")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .named(
                "mac",
                SyntaxShape::String,
                "The MAC address (node ID) used to generate v1 UUIDs. Required.",
                Some('m'),
            )
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Generate a v1 (timestamp and mac address based) UUID string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uuidv1"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        uuid(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a uuid v1 string",
            example: "random uuid1 -m 00:11:22:33:44:55",
            result: None,
        }]
    }
}

fn uuid(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let ts = Timestamp::now(uuid::timestamp::context::NoContext);
    let node_id = get_mac_address(engine_state, stack, call, span)?;
    let uuid = Uuid::new_v1(ts, &node_id);
    let uuid_str = uuid.hyphenated().to_string();

    Ok(PipelineData::Value(Value::string(uuid_str, span), None))
}

fn get_mac_address(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
) -> Result<[u8; 6], ShellError> {
    let mac_str: Option<String> = call.get_flag(engine_state, stack, "mac")?;

    let mac_str = match mac_str {
        Some(mac) => mac,
        None => {
            return Err(ShellError::MissingParameter {
                param_name: "mac".to_string(),
                span,
            });
        }
    };

    let mac_parts = mac_str.split(':').collect::<Vec<&str>>();
    if mac_parts.len() != 6 {
        return Err(ShellError::IncorrectValue {
            msg: "MAC address must be in the format XX:XX:XX:XX:XX:XX".to_string(),
            val_span: span,
            call_span: span,
        });
    }

    let mac: [u8; 6] = mac_parts
        .iter()
        .map(|x| u8::from_str_radix(x, 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| ShellError::IncorrectValue {
            msg: "MAC address must be in the format XX:XX:XX:XX:XX:XX".to_string(),
            val_span: span,
            call_span: span,
        })?
        .try_into()
        .map_err(|_| ShellError::IncorrectValue {
            msg: "MAC address must be in the format XX:XX:XX:XX:XX:XX".to_string(),
            val_span: span,
            call_span: span,
        })?;

    Ok(mac)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
