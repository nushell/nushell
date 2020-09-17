use super::matchers::Matcher;
use crate::completion::{Completer, Context, Suggestion};
use crate::context;

pub struct FlagCompleter {
    pub(crate) cmd: String,
}

impl Completer for FlagCompleter {
    fn complete(&self, ctx: &Context<'_>, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion> {
        let context: &context::Context = ctx.as_ref();

        if let Some(cmd) = context.registry.get_command(&self.cmd) {
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
