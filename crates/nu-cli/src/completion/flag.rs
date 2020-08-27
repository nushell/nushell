use crate::completion::matchers::Matcher;
use crate::completion::{Context, Suggestion};
use crate::context;

pub struct Completer;

impl Completer {
    pub fn complete(
        &self,
        ctx: &Context<'_>,
        cmd: String,
        partial: &str,
        matcher: &Box<dyn Matcher>,
    ) -> Vec<Suggestion> {
        let context: &context::Context = ctx.as_ref();

        if let Some(cmd) = context.registry.get_command(&cmd) {
            let sig = cmd.signature();
            let mut suggestions = Vec::new();
            for (name, (named_type, _desc)) in sig.named.iter() {
                suggestions.push(format!("--{}", name));

                if let Some(c) = named_type.get_short() {
                    suggestions.push(format!("-{}", c));
                }
            }

            suggestions
                .into_iter()
                .filter(|v| matcher.matches(partial, v))
                .map(|v| Suggestion {
                    replacement: format!("{} ", v),
                    display: v,
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
