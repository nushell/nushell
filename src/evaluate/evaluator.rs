use crate::object::base as obj;
use crate::parser::ast;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
crate struct Scope {
    it: Value,
}

impl Scope {
    crate fn empty() -> Scope {
        Scope {
            it: Value::nothing(),
        }
    }
}

crate fn evaluate_expr(expr: &ast::Expression, scope: &Scope) -> Result<Value, ShellError> {
    use ast::*;

    match expr {
        Expression::Leaf(l) => Ok(evaluate_leaf(l)),
        Expression::Parenthesized(p) => evaluate_expr(&p.expr, scope),
        Expression::Block(b) => evaluate_block(&b, scope),
        Expression::Path(p) => evaluate_path(&p, scope),
        Expression::Binary(b) => evaluate_binary(b, scope),
        Expression::VariableReference(r) => evaluate_reference(r, scope),
    }
}

fn evaluate_leaf(leaf: &ast::Leaf) -> Value {
    use ast::*;

    match leaf {
        Leaf::String(s) => Value::string(s),
        Leaf::Bare(s) => Value::string(s),
        Leaf::Boolean(b) => Value::boolean(*b),
        Leaf::Int(i) => Value::int(*i),
    }
}

fn evaluate_reference(r: &ast::Variable, scope: &Scope) -> Result<Value, ShellError> {
    use ast::Variable::*;

    match r {
        It => Ok(scope.it.copy()),
        True => Ok(Value::boolean(true)),
        False => Ok(Value::boolean(false)),
        Other(s) => Err(ShellError::string(&format!(
            "Unimplemented variable reference: {}",
            s
        ))),
    }
}

fn evaluate_binary(binary: &ast::Binary, scope: &Scope) -> Result<Value, ShellError> {
    let left = evaluate_expr(&binary.left, scope)?;
    let right = evaluate_expr(&binary.right, scope)?;

    match left.compare(binary.operator, &right) {
        Some(v) => Ok(Value::boolean(v)),
        None => Err(ShellError::string(&format!(
            "Unimplemented evaluate_binary:\n{:#?}",
            binary
        ))),
    }
}

fn evaluate_block(block: &ast::Block, _scope: &Scope) -> Result<Value, ShellError> {
    Ok(Value::block(block.expr.clone()))
}

fn evaluate_path(path: &ast::Path, scope: &Scope) -> Result<Value, ShellError> {
    let head = path.head();
    let mut value = &evaluate_expr(head, scope)?;

    for name in path.tail() {
        let next = value.get_data_by_key(&name);

        match next {
            None => {
                return Err(ShellError::string(&format!(
                    "No key {} found in {}",
                    name,
                    path.print(),
                )))
            }
            Some(v) => value = v,
        }
    }

    Ok(value.copy())
}
