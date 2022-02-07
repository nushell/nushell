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
