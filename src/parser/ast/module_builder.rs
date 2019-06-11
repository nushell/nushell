use crate::parser::ast::module::{Item, Module};
use crate::parser::ast::{self, module};
use crate::parser::lexer::{Span, Spanned};
use derive_new::new;

#[derive(new)]
pub struct ModuleBuilder {
    #[new(default)]
    pos: usize,
}

#[allow(unused)]
impl ModuleBuilder {
    // crate fn function(&mut self, input: ) -> Function {}

    crate fn spanned_function(
        input: (
            Spanned<ast::Bare>,
            Vec<module::FormalParameter>,
            Option<Spanned<module::Type>>,
            Spanned<ast::Block>,
        ),
        start: usize,
        end: usize,
    ) -> module::Function {
        module::Function::new(input.0, input.1, input.2.map(Box::new), input.3)
    }

    crate fn spanned_formal_parameter(
        input: (ast::ParameterIdentifier, Spanned<module::Type>),
        start: usize,
        end: usize,
    ) -> module::FormalParameter {
        module::FormalParameter::new(input.0, input.1, Span::from((start, end)))
    }

    crate fn items(&mut self, input: Vec<&dyn Fn(&mut ModuleBuilder) -> Item>) -> Module {
        let start = self.pos;

        let mut items = vec![];
        let mut input = input.into_iter();

        let next = input.next().unwrap();
        items.push(next(self));

        for item in input {
            self.consume(" | ");
            items.push(item(self));
        }

        let end = self.pos;

        ModuleBuilder::spanned_items(items, start, end)
    }

    pub fn spanned_items(input: Vec<Item>, start: usize, end: usize) -> ast::Module {
        ast::Module::new(Span::from((start, end)), input)
    }

    pub fn sp(&mut self) {
        self.consume(" ");
    }

    pub fn ws(&mut self, input: &str) {
        self.consume(input);
    }

    pub fn newline(&mut self) {
        self.consume("\n");
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        (start, self.pos)
    }
}
