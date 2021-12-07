use crate::Example;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, SyntaxShape, Value};

impl Plugin for Example {
    fn signature(&self) -> Vec<Signature> {
        // It is possible to declare multiple signature in a plugin
        // Each signature will be converted to a command declaration once the
        // plugin is registered to nushell
        vec![
            Signature::build("test-1")
                .desc("Signature test 1 for plugin. Returns Value::Nothing")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .category(Category::Experimental),
            Signature::build("test-2")
                .desc("Signature test 2 for plugin. Returns list of records")
                .required("a", SyntaxShape::Int, "required integer value")
                .required("b", SyntaxShape::String, "required string value")
                .switch("flag", "a flag for the signature", Some('f'))
                .optional("opt", SyntaxShape::Int, "Optional number")
                .named("named", SyntaxShape::String, "named string", Some('n'))
                .rest("rest", SyntaxShape::String, "rest value string")
                .category(Category::Experimental),
            Signature::build("test-3")
                .desc("Signature test 3 for plugin. Returns labeled error")
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
            "test-1" => self.test1(call, input),
            "test-2" => self.test2(call, input),
            "test-3" => self.test3(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "using the wrong signature".into(),
                span: Some(call.head),
            }),
        }
    }
}
