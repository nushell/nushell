use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc, time::Instant};

use crate::{
    parser::Operator, Block, BlockId, Call, Expr, Expression, ParserState, Span, Statement, VarId,
};

#[derive(Debug)]
pub enum ShellError {
    Mismatch(String, Span),
    Unsupported(Span),
    InternalError(String),
    VariableNotFound(Span),
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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => lhs == rhs,
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => lhs == rhs,
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => lhs == rhs,
            (Value::List(l1), Value::List(l2)) => l1 == l2,
            (Value::Block(b1), Value::Block(b2)) => b1 == b2,
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool { val, .. } => {
                write!(f, "{}", val)
            }
            Value::Int { val, .. } => {
                write!(f, "{}", val)
            }
            Value::String { val, .. } => write!(f, "{}", val),
            Value::List(..) => write!(f, "<list>"),
            Value::Block(..) => write!(f, "<block>"),
            Value::Unknown => write!(f, "<unknown>"),
        }
    }
}

impl Value {
    pub fn add(&self, rhs: &Value) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs + rhs,
                span: Span::unknown(),
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::String {
                val: lhs.to_string() + rhs,
                span: Span::unknown(),
            }),

            _ => Ok(Value::Unknown),
        }
    }
}

pub struct State<'a> {
    pub parser_state: &'a ParserState,
}

pub struct StackFrame {
    pub vars: HashMap<VarId, Value>,
    pub parent: Option<Stack>,
}

#[derive(Clone)]
pub struct Stack(Rc<RefCell<StackFrame>>);

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Stack {
        Stack(Rc::new(RefCell::new(StackFrame {
            vars: HashMap::new(),
            parent: None,
        })))
    }
    pub fn get_var(&self, var_id: VarId) -> Result<Value, ShellError> {
        let this = self.0.borrow();
        match this.vars.get(&var_id) {
            Some(v) => Ok(v.clone()),
            _ => {
                if let Some(parent) = &this.parent {
                    parent.get_var(var_id)
                } else {
                    Err(ShellError::InternalError("variable not found".into()))
                }
            }
        }
    }

    pub fn add_var(&self, var_id: VarId, value: Value) {
        let mut this = self.0.borrow_mut();
        this.vars.insert(var_id, value);
    }

    pub fn enter_scope(self) -> Stack {
        Stack(Rc::new(RefCell::new(StackFrame {
            vars: HashMap::new(),
            parent: Some(self),
        })))
    }

    pub fn print_stack(&self) {
        println!("===frame===");
        for (var, val) in &self.0.borrow().vars {
            println!("{}: {:?}", var, val);
        }
        if let Some(parent) = &self.0.borrow().parent {
            parent.print_stack()
        }
    }
}

pub fn eval_operator(
    _state: &State,
    _stack: Stack,
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

fn eval_call(state: &State, stack: Stack, call: &Call) -> Result<Value, ShellError> {
    let decl = state.parser_state.get_decl(call.decl_id);
    if let Some(block_id) = decl.body {
        let stack = stack.enter_scope();
        for (arg, param) in call
            .positional
            .iter()
            .zip(decl.signature.required_positional.iter())
        {
            let result = eval_expression(state, stack.clone(), arg)?;
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            stack.add_var(var_id, result);
        }
        let block = state.parser_state.get_block(block_id);
        eval_block(state, stack, block)
    } else if decl.signature.name == "let" {
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");

        let rhs = eval_expression(state, stack.clone(), keyword_expr)?;

        //println!("Adding: {:?} to {}", rhs, var_id);

        stack.add_var(var_id, rhs);
        Ok(Value::Unknown)
    } else if decl.signature.name == "if" {
        let cond = &call.positional[0];
        let then_block = call.positional[1]
            .as_block()
            .expect("internal error: expected block");
        let else_case = call.positional.get(2);

        let result = eval_expression(state, stack.clone(), cond)?;
        match result {
            Value::Bool { val, .. } => {
                if val {
                    let block = state.parser_state.get_block(then_block);
                    let stack = stack.enter_scope();
                    eval_block(state, stack, block)
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let block = state.parser_state.get_block(block_id);
                            let stack = stack.enter_scope();
                            eval_block(state, stack, block)
                        } else {
                            eval_expression(state, stack, else_expr)
                        }
                    } else {
                        eval_expression(state, stack, else_case)
                    }
                } else {
                    Ok(Value::Unknown)
                }
            }
            _ => Err(ShellError::Mismatch("bool".into(), Span::unknown())),
        }
    } else if decl.signature.name == "build-string" {
        let mut output = vec![];

        for expr in &call.positional {
            let val = eval_expression(state, stack.clone(), expr)?;

            output.push(val.to_string());
        }
        Ok(Value::String {
            val: output.join(""),
            span: call.head,
        })
    } else if decl.signature.name == "benchmark" {
        let block = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let block = state.parser_state.get_block(block);

        let stack = stack.enter_scope();
        let start_time = Instant::now();
        eval_block(state, stack, block)?;
        let end_time = Instant::now();
        println!("{} ms", (end_time - start_time).as_millis());
        Ok(Value::Unknown)
    } else if decl.signature.name == "for" {
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");
        let end_val = eval_expression(state, stack.clone(), keyword_expr)?;

        let block = call.positional[2]
            .as_block()
            .expect("internal error: expected block");
        let block = state.parser_state.get_block(block);

        let stack = stack.enter_scope();

        let mut x = Value::Int {
            val: 0,
            span: Span::unknown(),
        };

        loop {
            if x == end_val {
                break;
            } else {
                stack.add_var(var_id, x.clone());
                eval_block(state, stack.clone(), block)?;
            }
            if let Value::Int { ref mut val, .. } = x {
                *val += 1
            }
        }
        Ok(Value::Unknown)
    } else {
        Ok(Value::Unknown)
    }
}

pub fn eval_expression(
    state: &State,
    stack: Stack,
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
        Expr::Var(var_id) => stack
            .get_var(*var_id)
            .map_err(move |_| ShellError::VariableNotFound(expr.span)),
        Expr::Call(call) => eval_call(state, stack, call),
        Expr::ExternalCall(_, _) => Err(ShellError::Unsupported(expr.span)),
        Expr::Operator(_) => Ok(Value::Unknown),
        Expr::BinaryOp(lhs, op, rhs) => {
            let lhs = eval_expression(state, stack.clone(), lhs)?;
            let op = eval_operator(state, stack.clone(), op)?;
            let rhs = eval_expression(state, stack, rhs)?;

            match op {
                Operator::Plus => lhs.add(&rhs),
                _ => Ok(Value::Unknown),
            }
        }

        Expr::Subexpression(block_id) => {
            let block = state.parser_state.get_block(*block_id);

            let stack = stack.enter_scope();
            eval_block(state, stack, block)
        }
        Expr::Block(block_id) => Ok(Value::Block(*block_id)),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(state, stack.clone(), expr)?);
            }
            Ok(Value::List(output))
        }
        Expr::Table(_, _) => Err(ShellError::Unsupported(expr.span)),
        Expr::Keyword(_, _, expr) => eval_expression(state, stack, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Signature(_) => Ok(Value::Unknown),
        Expr::Garbage => Ok(Value::Unknown),
    }
}

pub fn eval_block(state: &State, stack: Stack, block: &Block) -> Result<Value, ShellError> {
    let mut last = Ok(Value::Unknown);

    for stmt in &block.stmts {
        if let Statement::Expression(expression) = stmt {
            last = Ok(eval_expression(state, stack.clone(), expression)?);
        }
    }

    last
}
