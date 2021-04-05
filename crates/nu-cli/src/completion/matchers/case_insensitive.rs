use crate::completion::matchers;
pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool {
        from.to_ascii_lowercase()
            .starts_with(partial.to_ascii_lowercase().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: check some Unicode matches if this becomes relevant

    // FIXME: could work exhaustively through ['-', '--'. ''] in a loop for each test
    #[test]
    fn completes_exact_matches() {
        let matcher: Box<dyn matchers::Matcher> = Box::new(Matcher);

        assert!(matcher.matches("shouldmatch", "shouldmatch"));
        assert!(matcher.matches("shouldm", "shouldmatch"));
        assert!(matcher.matches("--also-should-m", "--also-should-match"));
        assert!(matcher.matches("-also-should-m", "-also-should-match"));
    }

    #[test]
    fn completes_case_insensitive_matches() {
        let matcher: Box<dyn matchers::Matcher> = Box::new(Matcher);

        assert!(matcher.matches("thisshould", "Thisshouldmatch"));
        assert!(matcher.matches("--Shouldm", "--shouldmatch"));
        assert!(matcher.matches("-Shouldm", "-shouldmatch"));
    }

    #[test]
    fn should_not_match_when_unequal() {
        let matcher: Box<dyn matchers::Matcher> = Box::new(Matcher);

        assert!(!matcher.matches("ashouldmatch", "Shouldnotmatch"));
        assert!(!matcher.matches("--ashouldnotmatch", "--Shouldnotmatch"));
        assert!(!matcher.matches("-ashouldnotmatch", "-Shouldnotmatch"));
    }
}
