use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, StateWorkingSet, SyntaxShape, Type,
    Value,
};

// command to manipulate scope ($nu.scope) for a particular function
// because custom commands don't populate signature, category and many other scope fields
// this utility lets you put whatever you want into any field of scope *for any command*.  Use with care.

#[derive(Clone)]
pub struct ScopeHack;

impl Command for ScopeHack {
    fn name(&self) -> &str {
        "scope-hack"
    }

    fn usage(&self) -> &str {
        "Manipulate scope for existing defined function."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("scope-hack")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["experimental"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::boolean(do_scope_hack(engine_state), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update the scope definition for command 'victim_def' for now, eventually for any specified command.",
                example: r#"if (scope-hack) {"Changes made"} else {"No change to scope"}"#,
                result: Some(Value::test_string("Changes made")),
            },
        ]
    }
}

fn do_scope_hack(engine_state: &EngineState) -> bool // True if existing definition was updated
 {
    let command = "victim_def";
    //let mut working_set = StateWorkingSet::new(&engine_state);

    let desired_sig = Signature::new(command.as_bytes())
        .usage("sample custom command with signature")
        .input_output_types(vec![(Type::String, Type::String)])
        .required("a", SyntaxShape::Int, "first required is an int")
        .named("c_flag", SyntaxShape::String, "optional flag", Some('c'))
        .category(nu_protocol::Category::Conversions);

    // find command if already defined in existing scope.

    true
}
