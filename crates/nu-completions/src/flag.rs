use super::matchers::Matcher;
use crate::{Completer, CompletionContext, Suggestion};

pub struct FlagCompleter {
    pub(crate) cmd: String,
}

impl<Context> Completer<Context> for FlagCompleter
where
    Context: CompletionContext,
{
    fn complete(&self, ctx: &Context, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion> {
        if let Some(sig) = ctx.signature_registry().get(&self.cmd) {
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
