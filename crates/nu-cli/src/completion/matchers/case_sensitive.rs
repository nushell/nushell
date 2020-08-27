use crate::completion::matchers;

pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool {
        from.starts_with(partial)
    }
}
