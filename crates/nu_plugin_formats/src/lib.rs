mod from;

use from::{eml, ics, ini, vcf};
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, SyntaxShape, Type, Value};

pub struct FromCmds;

impl Plugin for FromCmds {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![
            PluginSignature::build(eml::CMD_NAME)
                .input_output_types(vec![(Type::String, Type::Record(vec![]))])
                .named(
                    "preview-body",
                    SyntaxShape::Int,
                    "How many bytes of the body to preview",
                    Some('b'),
                )
                .plugin_examples(eml::examples())
                .category(Category::Formats),
            PluginSignature::build(ics::CMD_NAME)
                .input_output_types(vec![(Type::String, Type::Table(vec![]))])
                .plugin_examples(ics::examples())
                .category(Category::Formats),
            PluginSignature::build(vcf::CMD_NAME)
                .input_output_types(vec![(Type::String, Type::Table(vec![]))])
                .plugin_examples(vcf::examples())
                .category(Category::Formats),
            PluginSignature::build(ini::CMD_NAME)
                .input_output_types(vec![(Type::String, Type::Record(vec![]))])
                .plugin_examples(ini::examples())
                .category(Category::Formats),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            eml::CMD_NAME => eml::from_eml_call(call, input),
            ics::CMD_NAME => ics::from_ics_call(call, input),
            vcf::CMD_NAME => vcf::from_vcf_call(call, input),
            ini::CMD_NAME => ini::from_ini_call(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
