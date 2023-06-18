use std::sync::{Arc, Mutex};

use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, LazyRecord, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LazyMake;

impl Command for LazyMake {
    fn name(&self) -> &str {
        "lazy make"
    }

    fn signature(&self) -> Signature {
        Signature::build("lazy make")
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![]))])
            .required_named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Closure that gets called when the LazyRecord needs to list the available column names",
                Some('c')
            )
            .required_named(
                "get-value",
                SyntaxShape::Closure(Some(vec![SyntaxShape::String])),
                "Closure to call when a value needs to be produced on demand",
                Some('g')
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Create a lazy record."
    }

    fn extra_usage(&self) -> &str {
        "Lazy records are special records that only evaluate their values once the property is requested.
        For example, when printing a lazy record, all of its fields will be collected. But when accessing
        a specific property, only it will be evaluated.

        Note that this is unrelated to the lazyframes feature bundled with dataframes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["deferred", "record", "procedural"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let columns: Vec<String> = call
            .get_flag(engine_state, stack, "columns")?
            .expect("required flag");
        let get_value: Closure = call
            .get_flag(engine_state, stack, "get-value")?
            .expect("required flag");

        Ok(Value::LazyRecord {
            val: Box::new(NuLazyRecord {
                engine_state: engine_state.clone(),
                stack: Arc::new(Mutex::new(stack.clone())),
                columns,
                get_value,
                span,
            }),
            span,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // TODO: Figure out how to "test" these examples, or leave results as None
            Example {
                description: "Create a lazy record",
                example: r#"lazy make --columns ["haskell", "futures", "nushell"] --get-value { |lazything| $lazything + "!" }"#,
                result: None,
            },
            Example {
                description: "Test the laziness of lazy records",
                example: r#"lazy make -c ["hello"] -g { |key| print $"getting ($key)!"; $key | str upcase }"#,
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
struct NuLazyRecord {
    engine_state: EngineState,
    stack: Arc<Mutex<Stack>>,
    columns: Vec<String>,
    get_value: Closure,
    span: Span,
}

impl std::fmt::Debug for NuLazyRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NuLazyRecord").finish()
    }
}

impl<'a> LazyRecord<'a> for NuLazyRecord {
    fn column_names(&'a self) -> Vec<&'a str> {
        self.columns.iter().map(|column| column.as_str()).collect()
    }

    fn get_column_value(&self, column: &str) -> Result<Value, ShellError> {
        let block = self.engine_state.get_block(self.get_value.block_id);
        let mut stack = self.stack.lock().expect("lock must not be poisoned");
        let column_value = Value::String {
            val: column.into(),
            span: self.span,
        };

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, column_value.clone());
            }
        }

        let pipeline_result = eval_block(
            &self.engine_state,
            &mut stack,
            block,
            PipelineData::Value(column_value, None),
            false,
            false,
        );

        pipeline_result.map(|data| match data {
            PipelineData::Value(value, ..) => value,
            // TODO: Proper error handling.
            _ => Value::Nothing { span: self.span },
        })
    }

    fn span(&self) -> Span {
        self.span
    }

    fn clone_value(&self, span: Span) -> Value {
        Value::LazyRecord {
            val: Box::new((*self).clone()),
            span,
        }
    }
}
