use nu_engine::{command_prelude::*, scope::ScopeData};

#[derive(Clone)]
pub struct ScopeVariables;

impl Command for ScopeVariables {
    fn name(&self) -> &str {
        "scope variables"
    }

    fn signature(&self) -> Signature {
        Signature::build("scope variables")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Output info on the variables in the current scope."
    }

    fn extra_description(&self) -> &str {
        "Lists variables that are available at runtime in the current stack and active overlays \
(locals and globals). Nested scopes such as `do`, `if`/`for` bodies, and custom commands include \
their locals while that scope is active; outer locals remain visible when the outer frame is still \
on the stack.

Closures only capture free variables that are referenced in the closure body. An outer local that \
is never mentioned is not captured, so after the defining scope ends it will not appear in \
`scope variables` when that closure runs. Mentioning the variable (for example `$a`) causes it to \
be captured and listed.

For example, this shows `$a` inside the `do` block, but not when the returned closure runs later:

    do {
      let a = 123
      scope variables | where name == '$a' | print
      {|| scope variables | where name == '$a' }
    } | let factory
    do $factory

Adding a reference to `$a` in the closure body captures it so it appears:

    {|| $a; scope variables | where name == '$a' }"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let mut scope_data = ScopeData::new(engine_state, stack);
        scope_data.populate_vars();
        Ok(Value::list(scope_data.collect_vars(head), head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show the variables in the current scope.",
            example: "scope variables",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ScopeVariables)
    }
}
