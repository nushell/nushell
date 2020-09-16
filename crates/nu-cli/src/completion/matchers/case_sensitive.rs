use crate::completion::matchers;

pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool {
        from.starts_with(partial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_case_sensitive() {
        let matcher: Box<dyn matchers::Matcher> = Box::new(Matcher);

        //Should match
        assert!(matcher.matches("shouldmatch", "shouldmatch"));
        assert!(matcher.matches("shouldm", "shouldmatch"));
        assert!(matcher.matches("--also-should-m", "--also-should-match"));
        assert!(matcher.matches("-also-should-m", "-also-should-match"));

        // Should not match
        assert!(!matcher.matches("--Shouldnot", "--shouldnotmatch"));
    }
}
