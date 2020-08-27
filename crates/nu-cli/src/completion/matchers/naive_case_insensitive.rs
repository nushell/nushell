use crate::completion::matchers;
pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool {
        from.to_lowercase().starts_with(&partial.to_lowercase())
    }
}
