use crate::completion::matchers;
use unicase::UniCase;
pub struct Matcher;

impl matchers::Matcher for Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool {
        let from_index = std::cmp::min(from.len(), partial.len());
        // let from_index = std::cmp::max(0, from_index);
        let from_u_substring: UniCase<&str> = UniCase::new(from[..from_index].into());
        let partial_u: UniCase<&str> = UniCase::new(partial.into());
        from_u_substring == partial_u
    }
}

#[cfg(test)]
// TODO: check some unicode matches if this becomes relevant

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
