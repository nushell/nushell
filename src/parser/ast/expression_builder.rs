use crate::parser::lexer::{Span, Spanned};
use derive_new::new;

use crate::parser::ast::{self, Expression, RawExpression};

#[derive(new)]
pub struct ExpressionBuilder {
    #[new(default)]
    pos: usize,
}

#[allow(unused)]
impl ExpressionBuilder {
    pub fn op(&mut self, input: impl Into<ast::Operator>) -> Spanned<ast::Operator> {
        let input = input.into();

        let (start, end) = self.consume(input.as_str());

        self.pos = end;

        ExpressionBuilder::spanned_op(input, start, end)
    }

    pub fn spanned_op(
        input: impl Into<ast::Operator>,
        start: usize,
        end: usize,
    ) -> Spanned<ast::Operator> {
        Spanned {
            span: Span::from((start, end)),
            item: input.into(),
        }
    }

    pub fn string(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();

        let (start, _) = self.consume("\"");
        self.consume(&input);
        let (_, end) = self.consume("\"");
        self.pos = end;

        ExpressionBuilder::spanned_string(input, start, end)
    }

    pub fn spanned_string(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(ast::Leaf::String(input)),
        }
    }

    pub fn bare(&mut self, input: impl Into<ast::Bare>) -> Expression {
        let input = input.into();

        let (start, end) = self.consume(input.body());
        self.pos = end;

        ExpressionBuilder::spanned_bare(input, start, end)
    }

    pub fn spanned_bare(input: impl Into<ast::Bare>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(ast::Leaf::Bare(input)),
        }
    }

    pub fn boolean(&mut self, input: impl Into<bool>) -> Expression {
        let boolean = input.into();

        let (start, end) = match boolean {
            true => self.consume("$yes"),
            false => self.consume("$no"),
        };

        self.pos = end;

        ExpressionBuilder::spanned_boolean(boolean, start, end)
    }

    pub fn spanned_boolean(input: impl Into<bool>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(ast::Leaf::Boolean(input)),
        }
    }

    pub fn int(&mut self, input: impl Into<i64>) -> Expression {
        let int = input.into();

        let (start, end) = self.consume(&int.to_string());
        self.pos = end;

        ExpressionBuilder::spanned_int(int, start, end)
    }

    pub fn spanned_int(input: impl Into<i64>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(ast::Leaf::Int(input)),
        }
    }

    pub fn unit(&mut self, input: (impl Into<i64>, impl Into<ast::Unit>)) -> Expression {
        let (int, unit) = (input.0.into(), input.1.into());

        let (start, _) = self.consume(&int.to_string());
        let (_, end) = self.consume(&unit.to_string());
        self.pos = end;

        ExpressionBuilder::spanned_unit((int, unit), start, end)
    }

    pub fn spanned_unit(
        input: (impl Into<i64>, impl Into<ast::Unit>),
        start: usize,
        end: usize,
    ) -> Expression {
        let (int, unit) = (input.0.into(), input.1.into());

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(ast::Leaf::Unit(int, unit)),
        }
    }

    pub fn flag(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();

        let (start, _) = self.consume("--");
        let (_, end) = self.consume(&input);
        self.pos = end;

        ExpressionBuilder::spanned_flag(input, start, end)
    }

    pub fn spanned_flag(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Flag(ast::Flag::Longhand(input)),
        }
    }

    pub fn shorthand(&mut self, input: impl Into<String>) -> Expression {
        let int = input.into();

        let size = int.to_string().len();

        let start = self.pos;
        let end = self.pos + size + 1;
        self.pos = end;

        ExpressionBuilder::spanned_shorthand(int, start, end)
    }

    pub fn spanned_shorthand(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Flag(ast::Flag::Shorthand(input)),
        }
    }

    pub fn parens(
        &mut self,
        input: impl FnOnce(&mut ExpressionBuilder) -> Expression,
    ) -> Expression {
        let (start, _) = self.consume("(");
        let input = input(self);
        let (_, end) = self.consume(")");
        self.pos = end;

        ExpressionBuilder::spanned_parens(input, start, end)
    }

    pub fn spanned_parens(input: Expression, start: usize, end: usize) -> Expression {
        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Parenthesized(Box::new(ast::Parenthesized::new(input))),
        }
    }

    pub fn raw_block(
        &mut self,
        input: &dyn Fn(&mut ExpressionBuilder) -> Expression,
    ) -> Spanned<ast::Block> {
        let (start, _) = self.consume("{ ");
        let input = input(self);
        let (_, end) = self.consume(" }");
        self.pos = end;

        ExpressionBuilder::spanned_raw_block(input, start, end)
    }

    pub fn spanned_raw_block(input: Expression, start: usize, end: usize) -> Spanned<ast::Block> {
        Spanned::from_item(ast::Block::new(input), (start, end))
    }

    pub fn block(&mut self, input: &dyn Fn(&mut ExpressionBuilder) -> Expression) -> Expression {
        let (start, _) = self.consume("{ ");
        let input = input(self);
        let (_, end) = self.consume(" }");
        self.pos = end;

        ExpressionBuilder::spanned_block(input, start, end)
    }

    pub fn spanned_block(input: Expression, start: usize, end: usize) -> Expression {
        let block = ast::Block::new(input);

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Block(Box::new(block)),
        }
    }

    pub fn binary(
        &mut self,
        input: (
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
            &dyn Fn(&mut ExpressionBuilder) -> Spanned<ast::Operator>,
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
        ),
    ) -> Expression {
        let start = self.pos;

        let left = (input.0)(self);
        self.consume(" ");
        let operator = (input.1)(self);
        self.consume(" ");
        let right = (input.2)(self);

        let end = self.pos;

        ExpressionBuilder::spanned_binary((left, operator, right), start, end)
    }

    pub fn spanned_binary(
        input: (
            impl Into<Expression>,
            impl Into<Spanned<ast::Operator>>,
            impl Into<Expression>,
        ),
        start: usize,
        end: usize,
    ) -> Expression {
        let binary = ast::Binary::new(input.0, input.1.into(), input.2);

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Binary(Box::new(binary)),
        }
    }

    pub fn path(
        &mut self,
        input: (
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
            Vec<impl Into<String>>,
        ),
    ) -> Expression {
        let start = self.pos;

        let head = (input.0)(self);

        let mut tail = vec![];

        for item in input.1 {
            self.consume(".");
            let item = item.into();
            let (start, end) = self.consume(&item);
            tail.push(Spanned::new(Span::from((start, end)), item));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_path((head, tail), start, end)
    }

    pub fn spanned_path(
        input: (impl Into<Expression>, Vec<Spanned<String>>),
        start: usize,
        end: usize,
    ) -> Expression {
        let path = ast::Path::new(input.0.into(), input.1);

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Path(Box::new(path)),
        }
    }

    pub fn call(
        &mut self,
        input: (
            &(dyn Fn(&mut ExpressionBuilder) -> Expression),
            Vec<&dyn Fn(&mut ExpressionBuilder) -> Expression>,
        ),
    ) -> Expression {
        let start = self.pos;

        let name = (&input.0)(self);

        let mut args = vec![];

        for item in input.1 {
            self.consume(" ");
            args.push(item(self));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_call((name, args), start, end)
    }

    pub fn spanned_call(input: impl Into<ast::Call>, start: usize, end: usize) -> Expression {
        let call = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Call(Box::new(call)),
        }
    }

    pub fn var(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();
        let (start, _) = self.consume("$");
        let (_, end) = self.consume(&input);

        ExpressionBuilder::spanned_var(input, start, end)
    }

    pub fn spanned_var(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        let expr = match &input[..] {
            "it" => RawExpression::VariableReference(ast::Variable::It),
            _ => RawExpression::VariableReference(ast::Variable::Other(input)),
        };

        Expression {
            span: Span::from((start, end)),
            expr,
        }
    }

    pub fn pipeline(
        &mut self,
        input: Vec<&dyn Fn(&mut ExpressionBuilder) -> Expression>,
    ) -> ast::Pipeline {
        let start = self.pos;

        let mut exprs = vec![];
        let mut input = input.into_iter();

        let next = input.next().unwrap();
        exprs.push(next(self));

        for item in input {
            self.consume(" | ");
            exprs.push(item(self));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_pipeline(exprs, start, end)
    }

    pub fn spanned_pipeline(input: Vec<Expression>, start: usize, end: usize) -> ast::Pipeline {
        ast::Pipeline {
            span: Span::from((start, end)),
            commands: input,
        }
    }

    pub fn sp(&mut self) {
        self.consume(" ");
    }

    pub fn ws(&mut self, input: &str) {
        self.consume(input);
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        (start, self.pos)
    }
}
