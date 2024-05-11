use nu_plugin::SimplePluginCommand;
use nu_protocol::{Category, Signature, Type, Value};

use crate::Mime;

pub struct MimeCommand;

impl SimplePluginCommand for MimeCommand {
    type Plugin = Mime;

    fn name(&self) -> &str {
        "mime"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Nothing, Type::String)
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Various commands for working with MIME/Media Types."
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: &nu_protocol::Value,
    ) -> Result<nu_protocol::Value, nu_protocol::LabeledError> {
        Ok(Value::string(engine.get_help()?, call.head))
    }
}
