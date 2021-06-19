use nu_protocol::hir::{
    Call, CommandSpecification, Expression, ExternalRedirection, SpannedExpression, Synthetic,
};
use nu_source::{IntoSpanned, Span, SpannedItem};

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

    pub fn build(&mut self) -> CommandSpecification {
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

        CommandSpecification {
            name: name.to_string(),
            name_span: Span::unknown(),
            args: Call {
                head: Box::new(
                    Expression::Synthetic(Synthetic::String(name.item))
                        .into_spanned(Span::unknown()),
                ),
                positional: Some(args),
                named: None,
                external_redirection: ExternalRedirection::Stdout,
                span: Span::unknown(),
            },
        }
    }
}
