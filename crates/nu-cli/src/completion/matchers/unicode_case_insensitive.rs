use unicase::UniCase;
use crate::completion::matchers;
pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches (
        &self,
        partial: &str,
        from: &str
    ) -> bool {
        let from_u_substring: UniCase<&str> = UniCase::new(from[0..std::cmp::min(from.len(), partial.len())].into());
        let partial_u: UniCase<&str> = UniCase::new(partial.into());
        return from_u_substring == partial_u;
    }
}

