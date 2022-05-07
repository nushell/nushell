use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{Expr, WindowSpec};

#[derive(Clone)]
pub struct OverExpr;

impl Command for OverExpr {
    fn name(&self) -> &str {
        "db over"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "partition-by",
                SyntaxShape::Any,
                "columns to partition the window function",
            )
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Adds a partition to an expression function"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a partition to a function expresssion",
            example: "db function avg col_a | db over col_b",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "column", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let partitions = ExprDb::extract_exprs(value)?;

        let mut expression = ExprDb::try_from_pipeline(input, call.head)?;
        match expression.as_mut() {
            Expr::Function(function) => {
                function.over = Some(WindowSpec {
                    partition_by: partitions,
                    order_by: Vec::new(),
                    window_frame: None,
                });
            }
            s => {
                return Err(ShellError::GenericError(
                    "Expression doesnt define a function".into(),
                    format!("Expected an expression with a function. Got {}", s),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}
