use crate::database::values::dsl::{ExprDb, SelectDb};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{SelectItem, Ident, ObjectName};

#[derive(Clone)]
pub struct ColExpr;

impl Command for ColExpr {
    fn name(&self) -> &str {
        "db col"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "column name")
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Creates column expression for database"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression",
            example: "col name_1",
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
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: Value = call.req(engine_state, stack, 0)?;

        let select = match name {
            Value::String { val, .. } if val == "*" => SelectItem::Wildcard,
            Value::String { val, .. } if val.contains('.') => {
                let values = val.split('.').map(|part| Ident {
                    value: part.to_string(),
                    quote_style: None
                }).collect::<Vec<Ident>>();
                
                SelectItem::QualifiedWildcard(ObjectName(values))
            }
            _ => {
                let expr = ExprDb::try_from_value(name)?;
                SelectItem::UnnamedExpr(expr.into_native())
            }
        };

        let selection: SelectDb = select.into(); 
        Ok(selection.into_value(call.head).into_pipeline_data())
    }
}
