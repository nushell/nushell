use std::collections::HashMap;

use crate::{
    parser::Operator, Block, BlockId, Call, Expr, Expression, ParserState, Span, Statement, VarId,
};

#[derive(Debug)]
pub enum ShellError {
    Mismatch(String, Span),
    Unsupported(Span),
    InternalError(String),
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool { val: bool, span: Span },
    Int { val: i64, span: Span },
    String { val: String, span: Span },
    List(Vec<Value>),
    Block(BlockId),
    Unknown,
}
impl Value {
    pub fn add(&self, rhs: &Value) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs + rhs,
                span: Span::unknown(),
            }),
            _ => Ok(Value::Unknown),
        }
    }
}

pub struct State<'a> {
    pub parser_state: &'a ParserState,
}

pub struct Stack {
    pub vars: HashMap<VarId, Value>,
}

impl Stack {
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        match self.vars.get(&var_id) {
            Some(v) => Ok(v.clone()),
            _ => {
                println!("var_id: {}", var_id);
                Err(ShellError::InternalError("variable not found".into()))
            }
        }
    }

    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        self.vars.insert(var_id, value);
    }
}

pub fn eval_operator(
    _state: &State,
    _stack: &mut Stack,
    op: &Expression,
) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(operator.clone()),
        Expression { span, .. } => Err(ShellError::Mismatch("operator".to_string(), *span)),
    }
}

fn eval_call(state: &State, stack: &mut Stack, call: &Call) -> Result<Value, ShellError> {
    let decl = state.parser_state.get_decl(call.decl_id);
    if let Some(block_id) = decl.body {
        for (arg, param) in call
            .positional
            .iter()
            .zip(decl.signature.required_positional.iter())
        {
            let result = eval_expression(state, stack, arg)?;
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            stack.add_var(var_id, result);
        }
        let block = state.parser_state.get_block(block_id);
        eval_block(state, stack, block)
    } else {
        if decl.signature.name == "let" {
            let var_id = call.positional[0]
                .as_var()
                .expect("internal error: missing variable");

            let keyword_expr = call.positional[1]
                .as_keyword()
                .expect("internal error: missing keyword");

            let rhs = eval_expression(state, stack, keyword_expr)?;

            println!("Adding: {:?} to {}", rhs, var_id);

            stack.add_var(var_id, rhs);
            Ok(Value::Unknown)
        } else {
            Ok(Value::Unknown)
        }
    }
}

pub fn eval_expression(
    state: &State,
    stack: &mut Stack,
    expr: &Expression,
) -> Result<Value, ShellError> {
    match &expr.expr {
        Expr::Bool(b) => Ok(Value::Bool {
            val: *b,
            span: expr.span,
        }),
        Expr::Int(i) => Ok(Value::Int {
            val: *i,
            span: expr.span,
        }),
        Expr::Var(var_id) => stack.get_var(*var_id),
        Expr::Call(call) => eval_call(state, stack, call),
        Expr::ExternalCall(_, _) => Err(ShellError::Unsupported(expr.span)),
        Expr::Operator(_) => Ok(Value::Unknown),
        Expr::BinaryOp(lhs, op, rhs) => {
            let lhs = eval_expression(state, stack, &lhs)?;
            let op = eval_operator(state, stack, &op)?;
            let rhs = eval_expression(state, stack, &rhs)?;

            match op {
                Operator::Plus => lhs.add(&rhs),
                _ => Ok(Value::Unknown),
            }
        }

        Expr::Subexpression(block_id) => {
            let block = state.parser_state.get_block(*block_id);

            eval_block(state, stack, block)
        }
        Expr::Block(block_id) => Ok(Value::Block(*block_id)),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(state, stack, expr)?);
            }
            Ok(Value::List(output))
        }
        Expr::Table(_, _) => Err(ShellError::Unsupported(expr.span)),
        Expr::Keyword(_, expr) => eval_expression(state, stack, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Signature(_) => Ok(Value::Unknown),
        Expr::Garbage => Ok(Value::Unknown),
    }
}

pub fn eval_block(state: &State, stack: &mut Stack, block: &Block) -> Result<Value, ShellError> {
    let mut last = Ok(Value::Unknown);

    for stmt in &block.stmts {
        match stmt {
            Statement::Expression(expression) => {
                last = Ok(eval_expression(state, stack, expression)?);
            }
            _ => {}
        }
    }

    last
}
