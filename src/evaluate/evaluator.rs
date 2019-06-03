use crate::object::Primitive;
use crate::parser::ast;
use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;

#[derive(new)]
crate struct Scope {
    it: Value,
    #[new(default)]
    vars: IndexMap<String, Value>,
}

impl Scope {
    crate fn empty() -> Scope {
        Scope {
            it: Value::nothing(),
            vars: IndexMap::new(),
        }
    }
}

crate fn evaluate_expr(expr: &ast::Expression, scope: &Scope) -> Result<Value, ShellError> {
    use ast::*;
    match expr {
        Expression::Leaf(l) => Ok(evaluate_leaf(l)),
        Expression::Parenthesized(p) => evaluate_expr(&p.expr, scope),
        Expression::Flag(f) => Ok(Value::Primitive(Primitive::String(f.print()))),
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
        Leaf::Bare(path) => Value::string(path.to_string()),
        Leaf::Boolean(b) => Value::boolean(*b),
        Leaf::Int(i) => Value::int(*i),
        Leaf::Unit(i, unit) => unit.compute(*i),
    }
}

fn evaluate_reference(r: &ast::Variable, scope: &Scope) -> Result<Value, ShellError> {
    use ast::Variable::*;

    match r {
        It => Ok(scope.it.copy()),
        Other(s) => Ok(scope
            .vars
            .get(s)
            .map(|v| v.copy())
            .unwrap_or_else(|| Value::nothing())),
    }
}

fn evaluate_binary(binary: &ast::Binary, scope: &Scope) -> Result<Value, ShellError> {
    let left = evaluate_expr(&binary.left, scope)?;
    let right = evaluate_expr(&binary.right, scope)?;

    match left.compare(binary.operator, &right) {
        Some(v) => Ok(Value::boolean(v)),
        None => Err(ShellError::TypeError(format!(
            "Can't compare {} and {}",
            left.type_name(),
            right.type_name()
        ))),
    }
}

fn evaluate_block(block: &ast::Block, _scope: &Scope) -> Result<Value, ShellError> {
    Ok(Value::block(block.expr.clone()))
}

fn evaluate_path(path: &ast::Path, scope: &Scope) -> Result<Value, ShellError> {
    let head = path.head();
    let mut value = &evaluate_expr(head, scope)?;
    let mut seen = vec![];

    for name in path.tail() {
        let next = value.get_data_by_key(&name);
        seen.push(name);

        match next {
            None => {
                return Err(ShellError::MissingProperty {
                    expr: path.print(),
                    subpath: itertools::join(seen, "."),
                });
            }
            Some(v) => value = v,
        }
    }

    Ok(value.copy())
}
