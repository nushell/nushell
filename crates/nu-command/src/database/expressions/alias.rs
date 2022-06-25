use crate::database::values::dsl::{ExprDb, SelectDb};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Ident, SelectItem};

#[derive(Clone)]
pub struct AliasExpr;

impl Command for AliasExpr {
    fn name(&self) -> &str {
        "as"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("alias", SyntaxShape::String, "alias name")
            .input_type(Type::Custom("db-expression".into()))
            .output_type(Type::Custom("db-expression".into()))
            .category(Category::Custom("db-expression".into()))
    }

    fn usage(&self) -> &str {
        "Creates an alias for a column selection"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates an alias for a column selection",
            example: "field name_a | as new_a | into nu",
            result: Some(Value::Record {
                cols: vec!["expression".into(), "alias".into()],
                vals: vec![
                    Value::Record {
                        cols: vec!["value".into(), "quoted_style".into()],
                        vals: vec![
                            Value::String {
                                val: "name_a".into(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "None".into(),
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["value".into(), "quoted_style".into()],
                        vals: vec![
                            Value::String {
                                val: "new_a".into(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "None".into(),
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "alias", "column"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;

        let value = input.into_value(call.head);
        if let Ok(expr) = ExprDb::try_from_value(&value) {
            alias_selection(expr.into_native().into(), alias, call)
        } else {
            let select = SelectDb::try_from_value(&value)?;
            alias_selection(select, alias, call)
        }
    }
}

fn alias_selection(
    select: SelectDb,
    alias: String,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let select = match select.into_native() {
        SelectItem::UnnamedExpr(expr) => SelectItem::ExprWithAlias {
            expr,
            alias: Ident {
                value: alias,
                quote_style: None,
            },
        },
        SelectItem::ExprWithAlias { expr, .. } => SelectItem::ExprWithAlias {
            expr,
            alias: Ident {
                value: alias,
                quote_style: None,
            },
        },
        select => select,
    };

    let select: SelectDb = select.into();
    Ok(select.into_value(call.head).into_pipeline_data())
}

#[cfg(test)]
mod test {
    use super::super::FieldExpr;
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![Box::new(AliasExpr {}), Box::new(FieldExpr {})])
    }
}
