use crate::parser::ast;
use crate::parser::lexer::{Span, Spanned};
use derive_new::new;

#[derive(new, Eq, PartialEq, Clone, Debug)]
pub struct Module {
    span: Span,
    items: Vec<Item>,
}

impl Module {
    #[allow(unused)]
    crate fn print(&self) -> String {
        let mut iter = self.items.iter();

        let first = iter.next().unwrap();
        let mut out = first.print();

        for item in iter {
            out.push_str(&format!("\n{}", item.print()));
        }

        out
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum RawItem {
    Expression(ast::Pipeline),

    #[allow(unused)]
    Function(Function),
}

impl RawItem {
    fn print(&self) -> String {
        match self {
            RawItem::Expression(p) => p.print(),
            RawItem::Function(f) => f.print(),
        }
    }
}

pub type Item = Spanned<RawItem>;

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Type {
    Any,
    Int,
    Decimal,
    Bytes,
    Text,
    Boolean,
    Date,
    Object,
    List,
    Block,
    // Object(IndexMap<Spanned<String>, Spanned<Type>>),
    // List(Box<Spanned<Type>>),
    // Block {
    //     arguments: Spanned<Vec<Spanned<Type>>>,
    //     return_type: Box<Spanned<Type>>,
    // },
}

impl Type {
    #[allow(unused)]
    crate fn print(&self) -> String {
        use Type::*;

        match self {
            Any => "any",
            Int => "int",
            Decimal => "decimal",
            Bytes => "bytes",
            Text => "text",
            Boolean => "boolean",
            Date => "date",
            Object => "object",
            List => "list",
            Block => "block",
        }
        .to_string()
    }
}

#[derive(Eq, PartialEq, Clone, Debug, new)]
pub struct FormalParameter {
    name: ast::ParameterIdentifier,
    ty: Spanned<Type>,
    span: Span,
}

#[derive(Eq, PartialEq, Clone, Debug, new)]
pub struct Function {
    name: Spanned<ast::Bare>,
    params: Vec<FormalParameter>,
    return_type: Option<Box<Spanned<Type>>>,
    body: Spanned<ast::Block>,
}

impl Function {
    crate fn print(&self) -> String {
        use pretty::{BoxDoc, Doc};

        let doc: Doc<BoxDoc<()>> = Doc::text("function")
            .append(Doc::space())
            .append(Doc::text(self.name.item.to_string()))
            // todo: signature
            .append(Doc::space())
            .append(Doc::text("{"))
            .append(Doc::newline())
            .append(Doc::text(self.body.print()).nest(1))
            .append(Doc::newline())
            .append(Doc::text("}"));

        format!("{}", doc.pretty(80))
    }
}
