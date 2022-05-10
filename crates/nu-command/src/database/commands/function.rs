use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{Expr, Function, FunctionArg, FunctionArgExpr, Ident, ObjectName};

#[derive(Clone)]
pub struct FunctionExpr;

impl Command for FunctionExpr {
    fn name(&self) -> &str {
        "db fn"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "function name")
            .switch("distinct", "distict values", Some('d'))
            .rest("arguments", SyntaxShape::Any, "function arguments")
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Creates function expression for a select operation"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a function expression",
            example: "db fn count name_1",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "function", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: String = call.req(engine_state, stack, 0)?;
        let vals: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(value)?;

        let name: Vec<Ident> = name
            .split('.')
            .map(|part| Ident {
                value: part.to_string(),
                quote_style: None,
            })
            .collect();
        let name = ObjectName(name);

        let args: Vec<FunctionArg> = expressions
            .into_iter()
            .map(|expr| {
                let arg = FunctionArgExpr::Expr(expr);

                FunctionArg::Unnamed(arg)
            })
            .collect();

        let expression: ExprDb = Expr::Function(Function {
            name,
            args,
            over: None,
            distinct: call.has_flag("distinct"),
        })
        .into();

        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}
