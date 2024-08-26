use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nu_parser::trim_quotes_str;
use nu_protocol::{CompletionAlgorithm, CompletionSort};
use nu_utils::IgnoreCaseExt;
use std::{borrow::Cow, fmt::Display};

use super::SemanticSuggestion;

/// Describes how suggestions should be matched.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MatchAlgorithm {
    /// Only show suggestions which begin with the given input
    ///
    /// Example:
    /// "git switch" is matched by "git sw"
    Prefix,

    /// Only show suggestions which contain the input chars at any place
    ///
    /// Example:
    /// "git checkout" is matched by "gco"
    Fuzzy,
}

pub struct NuMatcher<T> {
    options: CompletionOptions,
    needle: String,
    state: State<T>,
}

enum State<T> {
    Prefix {
        /// Holds (haystack, item)
        items: Vec<(String, T)>,
    },
    Fuzzy {
        /// Holds (haystack, item, score)
        items: Vec<(String, T, i64)>,
    },
}

/// Filters and sorts suggestions
impl<T> NuMatcher<T> {
    /// # Arguments
    ///
    /// * `needle` - The text to search for
    pub fn new(needle: impl AsRef<str>, options: CompletionOptions) -> NuMatcher<T> {
        let orig_needle = trim_quotes_str(needle.as_ref());
        let lowercase_needle = if options.case_sensitive {
            orig_needle.to_owned()
        } else {
            orig_needle.to_folded_case()
        };
        match options.match_algorithm {
            MatchAlgorithm::Prefix => NuMatcher {
                options,
                needle: lowercase_needle,
                state: State::Prefix { items: Vec::new() },
            },
            MatchAlgorithm::Fuzzy => NuMatcher {
                options,
                needle: orig_needle.to_owned(),
                state: State::Fuzzy { items: Vec::new() },
            },
        }
    }

    /// Add the given item if the given haystack matches.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: String, item: T) -> bool {
        match &mut self.state {
            State::Prefix { items } => {
                let haystack = trim_quotes_str(&haystack).to_owned();
                let haystack_lowercased = if self.options.case_sensitive {
                    Cow::Borrowed(&haystack)
                } else {
                    Cow::Owned(haystack.to_folded_case())
                };
                if haystack_lowercased.starts_with(self.needle.as_str()) {
                    items.push((haystack, item));
                    true
                } else {
                    false
                }
            }
            State::Fuzzy { items } => {
                let mut matcher = SkimMatcherV2::default();
                if self.options.case_sensitive {
                    matcher = matcher.respect_case();
                } else {
                    matcher = matcher.ignore_case();
                };
                let Some(score) = matcher.fuzzy_match(&haystack, &self.needle) else {
                    return false;
                };
                items.push((haystack, item, score));
                true
            }
        }
    }

    /// Remove the last added item. This is useful if you want to filter matched
    /// completions by some expensive condition. You can call `add`, run the expensive condition,
    /// then call `remove_last` if the expensive condition is false.
    pub fn remove_last(&mut self) {
        match &mut self.state {
            State::Prefix { items } => {
                items.pop();
            }
            State::Fuzzy { items } => {
                items.pop();
            }
        }
    }

    /// Get all the items that matched (sorted)
    pub fn results(self) -> Vec<T> {
        match self.state {
            State::Prefix { mut items, .. } => {
                items.sort_by(|(haystack1, _), (haystack2, _)| haystack1.cmp(haystack2));
                items.into_iter().map(|(_, item)| item).collect::<Vec<_>>()
            }
            State::Fuzzy { mut items, .. } => {
                match self.options.sort {
                    CompletionSort::Alphabetical => {
                        items.sort_by(|(haystack1, _, _), (haystack2, _, _)| {
                            haystack1.cmp(haystack2)
                        });
                    }
                    CompletionSort::Smart => {
                        items.sort_by(|(haystack1, _, score1), (haystack2, _, score2)| {
                            score2.cmp(score1).then(haystack1.cmp(haystack2))
                        });
                    }
                }
                items
                    .into_iter()
                    .map(|(_, item, _)| item)
                    .collect::<Vec<_>>()
            }
        }
    }
}

impl NuMatcher<SemanticSuggestion> {
    pub fn add_semantic_suggestion(&mut self, sugg: SemanticSuggestion) -> bool {
        let value = sugg.suggestion.value.to_string();
        self.add(value, sugg)
    }
}

impl From<CompletionAlgorithm> for MatchAlgorithm {
    fn from(value: CompletionAlgorithm) -> Self {
        match value {
            CompletionAlgorithm::Prefix => MatchAlgorithm::Prefix,
            CompletionAlgorithm::Fuzzy => MatchAlgorithm::Fuzzy,
        }
    }
}

impl TryFrom<String> for MatchAlgorithm {
    type Error = InvalidMatchAlgorithm;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "prefix" => Ok(Self::Prefix),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err(InvalidMatchAlgorithm::Unknown),
        }
    }
}

#[derive(Debug)]
pub enum InvalidMatchAlgorithm {
    Unknown,
}

impl Display for InvalidMatchAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            InvalidMatchAlgorithm::Unknown => write!(f, "unknown match algorithm"),
        }
    }
}

impl std::error::Error for InvalidMatchAlgorithm {}

#[derive(Clone)]
pub struct CompletionOptions {
    pub case_sensitive: bool,
    pub positional: bool,
    pub match_algorithm: MatchAlgorithm,
    pub sort: CompletionSort,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            positional: true,
            match_algorithm: MatchAlgorithm::Prefix,
            sort: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::{CompletionOptions, MatchAlgorithm, NuMatcher};

    #[rstest]
    #[case(MatchAlgorithm::Prefix, "example text", "", true)]
    #[case(MatchAlgorithm::Prefix, "example text", "examp", true)]
    #[case(MatchAlgorithm::Prefix, "example text", "text", false)]
    #[case(MatchAlgorithm::Fuzzy, "example text", "", true)]
    #[case(MatchAlgorithm::Fuzzy, "example text", "examp", true)]
    #[case(MatchAlgorithm::Fuzzy, "example text", "ext", true)]
    #[case(MatchAlgorithm::Fuzzy, "example text", "mplxt", true)]
    #[case(MatchAlgorithm::Fuzzy, "example text", "mpp", false)]
    fn match_algorithm_simple(
        #[case] match_algorithm: MatchAlgorithm,
        #[case] haystack: &str,
        #[case] needle: &str,
        #[case] should_match: bool,
    ) {
        let options = CompletionOptions {
            match_algorithm,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new(needle, options);
        matcher.add(haystack.to_string(), haystack);
        if should_match {
            assert_eq!(vec![haystack], matcher.results());
        } else {
            assert_ne!(vec![haystack], matcher.results());
        }
    }

    #[test]
    fn match_algorithm_fuzzy_sort_score() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fuzzy,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("fob", options);
        for item in ["foo/bar", "fob", "foo bar"] {
            matcher.add(item.to_string(), item);
        }
        // Sort by score, then in alphabetical order
        assert_eq!(vec!["fob", "foo bar", "foo/bar"], matcher.results());
    }
}
