use nu_parser::trim_quotes_str;
use nu_protocol::{CompletionAlgorithm, CompletionSort};
use nu_utils::IgnoreCaseExt;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};
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

    /// Only show suggestions which have a substring matching with the given input
    ///
    /// Example:
    /// "git checkout" is matched by "checkout"
    Substring,

    /// Only show suggestions which contain the input chars at any place
    ///
    /// Example:
    /// "git checkout" is matched by "gco"
    Fuzzy,
}

pub struct NuMatcher<'a, T> {
    options: &'a CompletionOptions,
    needle: String,
    state: State<T>,
}

enum State<T> {
    Prefix {
        /// Holds (haystack, item)
        items: Vec<(String, T)>,
    },
    Substring {
        /// Holds (haystack, item)
        items: Vec<(String, T)>,
    },
    Fuzzy {
        matcher: Matcher,
        atom: Atom,
        /// Holds (haystack, item, score)
        items: Vec<(String, T, u16)>,
    },
}

/// Filters and sorts suggestions
impl<T> NuMatcher<'_, T> {
    /// # Arguments
    ///
    /// * `needle` - The text to search for
    pub fn new(needle: impl AsRef<str>, options: &CompletionOptions) -> NuMatcher<T> {
        let needle = trim_quotes_str(needle.as_ref());
        match options.match_algorithm {
            MatchAlgorithm::Prefix => {
                let lowercase_needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_folded_case()
                };
                NuMatcher {
                    options,
                    needle: lowercase_needle,
                    state: State::Prefix { items: Vec::new() },
                }
            }
            MatchAlgorithm::Substring => {
                let lowercase_needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_folded_case()
                };
                NuMatcher {
                    options,
                    needle: lowercase_needle,
                    state: State::Substring { items: Vec::new() },
                }
            }
            MatchAlgorithm::Fuzzy => {
                let atom = Atom::new(
                    needle,
                    if options.case_sensitive {
                        CaseMatching::Respect
                    } else {
                        CaseMatching::Ignore
                    },
                    Normalization::Smart,
                    AtomKind::Fuzzy,
                    false,
                );
                NuMatcher {
                    options,
                    needle: needle.to_owned(),
                    state: State::Fuzzy {
                        matcher: Matcher::new({
                            let mut cfg = Config::DEFAULT;
                            cfg.prefer_prefix = true;
                            cfg
                        }),
                        atom,
                        items: Vec::new(),
                    },
                }
            }
        }
    }

    /// Returns whether or not the haystack matches the needle. If it does, `item` is added
    /// to the list of matches (if given).
    ///
    /// Helper to avoid code duplication between [NuMatcher::add] and [NuMatcher::matches].
    fn matches_aux(&mut self, haystack: &str, item: Option<T>) -> bool {
        let haystack = trim_quotes_str(haystack);
        match &mut self.state {
            State::Prefix { items } => {
                let haystack_folded = if self.options.case_sensitive {
                    Cow::Borrowed(haystack)
                } else {
                    Cow::Owned(haystack.to_folded_case())
                };
                let matches = haystack_folded.starts_with(self.needle.as_str());
                if matches {
                    if let Some(item) = item {
                        items.push((haystack.to_string(), item));
                    }
                }
                matches
            }
            State::Substring { items } => {
                let haystack_folded = if self.options.case_sensitive {
                    Cow::Borrowed(haystack)
                } else {
                    Cow::Owned(haystack.to_folded_case())
                };
                let matches = haystack_folded.contains(self.needle.as_str());
                if matches {
                    if let Some(item) = item {
                        items.push((haystack.to_string(), item));
                    }
                }
                matches
            }
            State::Fuzzy {
                matcher,
                atom,
                items,
            } => {
                let mut haystack_buf = Vec::new();
                let haystack_utf32 = Utf32Str::new(trim_quotes_str(haystack), &mut haystack_buf);
                let mut indices = Vec::new();
                let Some(score) = atom.indices(haystack_utf32, matcher, &mut indices) else {
                    return false;
                };
                if let Some(item) = item {
                    items.push((haystack.to_string(), item, score));
                }
                true
            }
        }
    }

    /// Add the given item if the given haystack matches the needle.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        self.matches_aux(haystack.as_ref(), Some(item))
    }

    /// Returns whether the haystack matches the needle.
    pub fn matches(&mut self, haystack: &str) -> bool {
        self.matches_aux(haystack, None)
    }

    /// Get all the items that matched (sorted)
    pub fn results(self) -> Vec<T> {
        match self.state {
            State::Prefix { mut items, .. } | State::Substring { mut items, .. } => {
                items.sort_by(|(haystack1, _), (haystack2, _)| {
                    let cmp_sensitive = haystack1.cmp(haystack2);
                    if self.options.case_sensitive {
                        cmp_sensitive
                    } else {
                        haystack1
                            .to_folded_case()
                            .cmp(&haystack2.to_folded_case())
                            .then(cmp_sensitive)
                    }
                });
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

impl NuMatcher<'_, SemanticSuggestion> {
    pub fn add_semantic_suggestion(&mut self, sugg: SemanticSuggestion) -> bool {
        let value = sugg.suggestion.value.to_string();
        self.add(value, sugg)
    }
}

impl From<CompletionAlgorithm> for MatchAlgorithm {
    fn from(value: CompletionAlgorithm) -> Self {
        match value {
            CompletionAlgorithm::Prefix => MatchAlgorithm::Prefix,
            CompletionAlgorithm::Substring => MatchAlgorithm::Substring,
            CompletionAlgorithm::Fuzzy => MatchAlgorithm::Fuzzy,
        }
    }
}

impl TryFrom<String> for MatchAlgorithm {
    type Error = InvalidMatchAlgorithm;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "prefix" => Ok(Self::Prefix),
            "substring" => Ok(Self::Substring),
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
    pub match_algorithm: MatchAlgorithm,
    pub sort: CompletionSort,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
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
    #[case(MatchAlgorithm::Substring, "example text", "", true)]
    #[case(MatchAlgorithm::Substring, "example text", "text", true)]
    #[case(MatchAlgorithm::Substring, "example text", "mplxt", false)]
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
        let mut matcher = NuMatcher::new(needle, &options);
        matcher.add(haystack, haystack);
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
        let mut matcher = NuMatcher::new("fob", &options);
        for item in ["foo/bar", "fob", "foo bar"] {
            matcher.add(item, item);
        }
        // Sort by score, then in alphabetical order
        assert_eq!(vec!["fob", "foo bar", "foo/bar"], matcher.results());
    }

    #[test]
    fn match_algorithm_fuzzy_sort_strip() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fuzzy,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("'love spaces' ", &options);
        for item in [
            "'i love spaces'",
            "'i love spaces' so much",
            "'lovespaces' ",
        ] {
            matcher.add(item, item);
        }
        // Make sure the spaces are respected
        assert_eq!(vec!["'i love spaces' so much"], matcher.results());
    }
}
