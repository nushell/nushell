use crate::evaluate::{evaluate_expr, Scope};
use crate::prelude::*;
use indexmap::IndexMap;

#[allow(unused)]
pub enum NamedType {
    Switch(String),
    Single(String),
    Array(String),
    Block(String),
}

pub enum PositionalType {
    Value(String),
    Block(String),
}

impl PositionalType {
    crate fn name(&self) -> String {
        match self {
            PositionalType::Value(s) => s.clone(),
            PositionalType::Block(s) => s.clone(),
        }
    }

    crate fn evaluate(&self, arg: ast::Expression, scope: &Scope) -> Result<Value, ShellError> {
        match self {
            PositionalType::Value(s) => evaluate_expr(&arg, scope),
            PositionalType::Block(s) => match arg {
                ast::Expression::Block(b) => Ok(Value::block(b.expr)),
                ast::Expression::Binary(b) => {
                    if let Some(s) = b.left.as_string() {
                        Ok(Value::block(ast::Expression::Binary(Box::new(
                            ast::Binary::new(
                                ast::Expression::Path(Box::new(ast::Path::new(
                                    ast::Expression::VariableReference(ast::Variable::It),
                                    vec![s],
                                ))),
                                b.operator,
                                b.right,
                            ),
                        ))))
                    } else {
                        Ok(Value::block(ast::Expression::Binary(b)))
                    }
                }
                other => Ok(Value::block(other)), // other =>
            },
        }
    }
}

#[allow(unused)]
pub struct CommandConfig {
    crate name: String,
    crate mandatory_positional: Vec<PositionalType>,
    crate optional_positional: Vec<PositionalType>,
    crate rest_positional: bool,
    crate named: IndexMap<String, NamedType>,
}

pub trait CommandRegistry {
    fn get(&self, name: &str) -> CommandConfig;
}
