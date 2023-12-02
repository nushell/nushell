use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, SyntaxShape, Value};

use crate::Example;

impl Plugin for Example {
    fn signature(&self) -> Vec<PluginSignature> {
        // It is possible to declare multiple signature in a plugin
        // Each signature will be converted to a command declaration once the
        // plugin is registered to nushell
        vec![
            PluginSignature::build("nu-example-1")
                .usage("PluginSignature test 1 for plugin. Returns Value::Nothing")
                .extra_usage("Extra usage for nu-example-1")
                .search_terms(vec!["example".into()])
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .plugin_examples(vec![PluginExample {
                    example: "nu-example-1 3 bb".into(),
                    description: "running example with an int value and string value".into(),
                    result: None,
                }])
                .category(Category::Experimental),
            PluginSignature::build("nu-example-2")
                .usage("PluginSignature test 2 for plugin. Returns list of records")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .category(Category::Experimental),
            PluginSignature::build("nu-example-3")
                .usage("PluginSignature test 3 for plugin. Returns labeled error")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .category(Category::Experimental),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        // You can use the name to identify what plugin signature was called
        match name {
            "nu-example-1" => self.test1(call, input),
            "nu-example-2" => self.test2(call, input),
            "nu-example-3" => self.test3(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin \
                      signature vector"
                    .into(),
                span: Some(call.head),
            }),
        }
    }
}
