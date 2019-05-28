use crate::evaluate::{evaluate_expr, Scope};
use crate::prelude::*;
use indexmap::IndexMap;

#[allow(unused)]
#[derive(Debug)]
pub enum NamedType {
    Switch(String),
    Single(String),
    Array(String),
    Block(String),
}

#[allow(unused)]
#[derive(Debug, Clone)]
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
            PositionalType::Value(_) => evaluate_expr(&arg, scope),
            PositionalType::Block(_) => match arg {
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

#[derive(Debug)]
pub struct CommandConfig {
    crate name: String,
    crate mandatory_positional: Vec<PositionalType>,
    crate optional_positional: Vec<PositionalType>,
    crate rest_positional: bool,
    crate named: IndexMap<String, NamedType>,
}

impl CommandConfig {
    crate fn evaluate_args(
        &self,
        mut args: impl Iterator<Item = &'expr ast::Expression>,
        scope: &Scope,
    ) -> Result<Vec<Value>, ShellError> {
        let mut results: Vec<Value> = vec![];

        for param in &self.mandatory_positional {
            let arg = args.next();

            let value = match arg {
                None => {
                    return Err(ShellError::string(format!(
                        "expected mandatory positional argument {}",
                        param.name()
                    )))
                }

                Some(arg) => param.evaluate(arg.clone(), scope)?,
            };

            results.push(value);
        }

        if self.rest_positional {
            let rest: Result<Vec<Value>, _> =
                args.map(|i| evaluate_expr(i, &Scope::empty())).collect();
            results.extend(rest?);
        } else {
            match args.next() {
                None => {}
                Some(_) => return Err(ShellError::string("Too many arguments")),
            }
        }

        Ok(results)
    }

    #[allow(unused)]
    crate fn signature(&self) -> String {
        format!("TODO")
    }
}

pub trait CommandRegistry {
    fn get(&self, name: &str) -> CommandConfig;
}
