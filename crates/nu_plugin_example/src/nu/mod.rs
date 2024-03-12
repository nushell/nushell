use crate::Example;
use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, SyntaxShape, Type, Value};

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
            PluginSignature::build("nu-example-config")
                .usage("Show plugin configuration")
                .extra_usage("The configuration is set under $env.config.plugins.example")
                .category(Category::Experimental)
                .search_terms(vec!["example".into(), "configuration".into()])
                .input_output_type(Type::Nothing, Type::Table(vec![])),
            PluginSignature::build("nu-example-env")
                .usage("Get environment variable(s)")
                .extra_usage("Returns all environment variables if no name provided")
                .category(Category::Experimental)
                .optional(
                    "name",
                    SyntaxShape::String,
                    "The name of the environment variable to get",
                )
                .switch("cwd", "Get current working directory instead", None)
                .search_terms(vec!["example".into(), "env".into()])
                .input_output_type(Type::Nothing, Type::Any),
            PluginSignature::build("nu-example-disable-gc")
                .usage("Disable the plugin garbage collector for `example`")
                .extra_usage(
                    "\
Plugins are garbage collected by default after a period of inactivity. This
behavior is configurable with `$env.config.plugin_gc.default`, or to change it
specifically for the example plugin, use
`$env.config.plugin_gc.plugins.example`.

This command demonstrates how plugins can control this behavior and disable GC
temporarily if they need to. It is still possible to stop the plugin explicitly
using `plugin stop example`.",
                )
                .search_terms(vec![
                    "example".into(),
                    "gc".into(),
                    "plugin_gc".into(),
                    "garbage".into(),
                ])
                .switch("reset", "Turn the garbage collector back on", None)
                .category(Category::Experimental),
        ]
    }

    fn run(
        &self,
        name: &str,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        // You can use the name to identify what plugin signature was called
        match name {
            "nu-example-1" => self.test1(call, input),
            "nu-example-2" => self.test2(call, input),
            "nu-example-3" => self.test3(call, input),
            "nu-example-config" => self.config(engine, call),
            "nu-example-env" => self.env(engine, call),
            "nu-example-disable-gc" => self.disable_gc(engine, call),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
