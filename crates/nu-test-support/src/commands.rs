<<<<<<< HEAD
use nu_protocol::hir::{Expression, ExternalArgs, ExternalCommand, SpannedExpression};
use nu_source::{Span, SpannedItem, Tag};

pub struct ExternalBuilder {
    name: String,
    args: Vec<String>,
}

impl ExternalBuilder {
    pub fn for_name(name: &str) -> ExternalBuilder {
        ExternalBuilder {
            name: name.to_string(),
            args: vec![],
        }
    }

    pub fn arg(&mut self, value: &str) -> &mut Self {
        self.args.push(value.to_string());
        self
    }

    pub fn build(&mut self) -> ExternalCommand {
        let mut path = crate::fs::binaries();
        path.push(&self.name);

        let name = path.to_string_lossy().to_string().spanned(Span::unknown());

        let args = self
            .args
            .iter()
            .map(|arg| SpannedExpression {
                expr: Expression::string(arg.to_string()),
                span: Span::unknown(),
            })
            .collect::<Vec<_>>();

        ExternalCommand {
            name: name.to_string(),
            name_tag: Tag::unknown(),
            args: ExternalArgs {
                list: args,
                span: name.span,
            },
        }
    }
}
=======
// use nu_protocol::{
//     ast::{Expr, Expression},
//     Span, Spanned, Type,
// };

// pub struct ExternalBuilder {
//     name: String,
//     args: Vec<String>,
// }

// impl ExternalBuilder {
//     pub fn for_name(name: &str) -> ExternalBuilder {
//         ExternalBuilder {
//             name: name.to_string(),
//             args: vec![],
//         }
//     }

//     pub fn arg(&mut self, value: &str) -> &mut Self {
//         self.args.push(value.to_string());
//         self
//     }

// pub fn build(&mut self) -> ExternalCommand {
//     let mut path = crate::fs::binaries();
//     path.push(&self.name);

//     let name = Spanned {
//         item: path.to_string_lossy().to_string(),
//         span: Span::new(0, 0),
//     };

//     let args = self
//         .args
//         .iter()
//         .map(|arg| Expression {
//             expr: Expr::String(arg.to_string()),
//             span: Span::new(0, 0),
//             ty: Type::Unknown,
//             custom_completion: None,
//         })
//         .collect::<Vec<_>>();

//     ExternalCommand {
//         name: name.to_string(),
//         name_tag: Tag::unknown(),
//         args: ExternalArgs {
//             list: args,
//             span: name.span,
//         },
//     }
// }
// }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
