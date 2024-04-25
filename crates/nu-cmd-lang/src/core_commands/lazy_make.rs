use nu_engine::{command_prelude::*, eval_block};
use nu_protocol::{debugger::WithoutDebug, engine::Closure, LazyRecord};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct LazyMake;

impl Command for LazyMake {
    fn name(&self) -> &str {
        "lazy make"
    }

    fn signature(&self) -> Signature {
        Signature::build("lazy make")
            .input_output_types(vec![(Type::Nothing, Type::record())])
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
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError {
                error: "Deprecated command".into(),
                msg: "warning: lazy records and the `lazy make` command will be removed in 0.94.0"
                    .into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            },
        );

        let span = call.head;
        let columns: Vec<Spanned<String>> = call
            .get_flag(engine_state, stack, "columns")?
            .expect("required flag");

        let get_value: Closure = call
            .get_flag(engine_state, stack, "get-value")?
            .expect("required flag");

        let mut unique = HashMap::with_capacity(columns.len());

        for col in &columns {
            match unique.entry(&col.item) {
                Entry::Occupied(entry) => {
                    return Err(ShellError::ColumnDefinedTwice {
                        col_name: col.item.clone(),
                        second_use: col.span,
                        first_use: *entry.get(),
                    });
                }
                Entry::Vacant(entry) => {
                    entry.insert(col.span);
                }
            }
        }

        let stack = stack.clone().reset_out_dest().capture();

        Ok(Value::lazy_record(
            Box::new(NuLazyRecord {
                engine_state: engine_state.clone(),
                stack: Arc::new(Mutex::new(stack)),
                columns: columns.into_iter().map(|s| s.item).collect(),
                get_value,
                span,
            }),
            span,
        )
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
                example: r#"lazy make --columns ["hello"] --get-value { |key| print $"getting ($key)!"; $key | str upcase }"#,
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
        let column_value = Value::string(column, self.span);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, column_value.clone());
            }
        }

        let pipeline_result = eval_block::<WithoutDebug>(
            &self.engine_state,
            &mut stack,
            block,
            PipelineData::Value(column_value, None),
        );

        pipeline_result.map(|data| match data {
            PipelineData::Value(value, ..) => value,
            // TODO: Proper error handling.
            _ => Value::nothing(self.span),
        })
    }

    fn span(&self) -> Span {
        self.span
    }

    fn clone_value(&self, span: Span) -> Value {
        Value::lazy_record(Box::new((*self).clone()), span)
    }
}
