use nu_parser::trim_quotes_str;
use nu_protocol::{levenshtein_distance, CompletionAlgorithm};
use nu_utils::IgnoreCaseExt;
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};
use std::{cmp::Ordering, fmt::Display, path::MAIN_SEPARATOR};

#[derive(Copy, Clone)]
pub enum SortBy {
    LevenshteinDistance,
    Ascending,
    None,
}

/// Describes how suggestions should be matched.
#[derive(Copy, Clone, Debug)]
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
    options: MatcherOptions,
    needle: String,
    state: State<T>,
}

enum State<T> {
    Prefix {
        /// Holds (haystack, item)
        items: Vec<(String, T)>,
    },
    Nucleo {
        matcher: Matcher,
        pat: Pattern,
        /// Holds (score, haystack, item, indices of matches)
        items: Vec<(u32, String, T, Vec<usize>)>,
    },
}

#[derive(Clone)]
pub struct MatcherOptions {
    pub match_algorithm: MatchAlgorithm,
    pub case_sensitive: bool,
    /// How to sort results. [`SortBy::None`] by default.
    pub sort_by: SortBy,
    /// When fuzzy matching, this will configure Nucleo to reward file paths.
    /// When sorting alphabetically, this will disregard trailing slashes.
    /// False by default.
    pub match_paths: bool,
}

impl<T> NuMatcher<T> {
    /// # Arguments
    ///
    /// * `needle` - The text to search for
    pub fn new(needle: impl AsRef<str>, options: MatcherOptions) -> NuMatcher<T> {
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
            MatchAlgorithm::Fuzzy => {
                let matcher = Matcher::new(if options.match_paths {
                    Config::DEFAULT.match_paths()
                } else {
                    Config::DEFAULT
                });
                let pat = Pattern::new(
                    // Use the original needle even if case sensitive, because Nucleo handles that
                    orig_needle,
                    if options.case_sensitive {
                        CaseMatching::Respect
                    } else {
                        CaseMatching::Ignore
                    },
                    Normalization::Smart,
                    AtomKind::Fuzzy,
                );
                NuMatcher {
                    options,
                    // Use lowercase needle here for Levenshtein distance comparison
                    needle: lowercase_needle,
                    state: State::Nucleo {
                        matcher,
                        pat,
                        items: Vec::new(),
                    },
                }
            }
        }
    }

    /// Add the given item if the given haystack matches.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        let haystack = haystack.as_ref();

        match &mut self.state {
            State::Prefix { items } => {
                let haystack = trim_quotes_str(haystack);
                let haystack = if self.options.case_sensitive {
                    haystack.to_owned()
                } else {
                    haystack.to_folded_case()
                };
                if haystack.starts_with(self.needle.as_str()) {
                    match self.options.sort_by {
                        SortBy::None => items.push((haystack, item)),
                        _ => {
                            let ind = match items.binary_search_by(|(other, _)| {
                                cmp(
                                    &self.needle,
                                    &self.options,
                                    other.as_str(),
                                    haystack.as_str(),
                                )
                            }) {
                                Ok(i) => i,
                                Err(i) => i,
                            };
                            items.insert(ind, (haystack, item));
                        }
                    }

                    true
                } else {
                    false
                }
            }
            State::Nucleo {
                matcher,
                pat,
                items,
            } => {
                let mut haystack_buf = Vec::new();
                let haystack_utf32 = Utf32Str::new(trim_quotes_str(haystack), &mut haystack_buf);
                // todo find out why nucleo uses u32 instead of usize for indices
                let mut indices = Vec::new();
                match pat.indices(haystack_utf32, matcher, &mut indices) {
                    Some(score) => {
                        indices.sort_unstable();
                        indices.dedup();

                        let match_record = (
                            score,
                            haystack.to_string(),
                            item,
                            indices.into_iter().map(|i| i as usize).collect(),
                        );
                        let ind =
                            match items.binary_search_by(|(other_score, other_haystack, _, _)| {
                                match self.options.sort_by {
                                    SortBy::None => score.cmp(other_score),
                                    _ => cmp(
                                        &self.needle,
                                        &self.options,
                                        other_haystack.as_str(),
                                        haystack,
                                    ),
                                }
                            }) {
                                Ok(i) => i,
                                Err(i) => i,
                            };
                        items.insert(ind, match_record);
                        true
                    }
                    None => false,
                }
            }
        }
    }

    /// Get all the items that matched
    pub fn get_results(self) -> Vec<T> {
        let (results, _): (Vec<_>, Vec<_>) = self.get_results_with_inds().into_iter().unzip();
        results
    }

    /// Get all the items that matched, along with the indices in their haystacks that matched
    pub fn get_results_with_inds(self) -> Vec<(T, Vec<usize>)> {
        match self.state {
            State::Prefix { items, .. } => items
                .into_iter()
                .map(|(_, item)| (item, (0..self.needle.len()).collect()))
                .collect(),
            State::Nucleo { items, .. } => items
                .into_iter()
                .map(|(_, _, items, indices)| (items, indices))
                .collect(),
        }
    }
}

fn cmp(needle: &str, options: &MatcherOptions, a: &str, b: &str) -> Ordering {
    match options.sort_by {
        SortBy::LevenshteinDistance => {
            let a_distance = levenshtein_distance(needle, a);
            let b_distance = levenshtein_distance(needle, b);
            a_distance.cmp(&b_distance)
        }
        SortBy::Ascending => {
            if options.match_paths {
                a.trim_end_matches(MAIN_SEPARATOR)
                    .cmp(b.trim_end_matches(MAIN_SEPARATOR))
            } else {
                a.cmp(b)
            }
        }
        SortBy::None => Ordering::Equal,
    }
}

impl MatcherOptions {
    pub fn new(completion_options: &CompletionOptions) -> Self {
        MatcherOptions {
            match_algorithm: completion_options.match_algorithm,
            case_sensitive: completion_options.case_sensitive,
            sort_by: SortBy::None,
            match_paths: false,
        }
    }

    pub fn sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    pub fn match_paths(mut self, match_paths: bool) -> Self {
        self.match_paths = match_paths;
        self
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
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            positional: true,
            match_algorithm: MatchAlgorithm::Prefix,
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::{CompletionOptions, MatchAlgorithm, MatcherOptions, NuMatcher};

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
        let options = MatcherOptions::new(&CompletionOptions {
            match_algorithm,
            case_sensitive: true,
            positional: false,
        });

        let mut matcher = NuMatcher::new(needle, options);
        matcher.add(haystack, haystack);
        if should_match {
            assert_eq!(vec![haystack], matcher.get_results());
        } else {
            assert_ne!(vec![haystack], matcher.get_results());
        }
    }

    #[test]
    fn match_algorithm_fuzzy_sort_score() {
        let options = MatcherOptions::new(&CompletionOptions {
            match_algorithm: MatchAlgorithm::Fuzzy,
            case_sensitive: true,
            positional: false,
        });

        // Taken from the nucleo-matcher crate's examples
        // todo more thorough tests
        let mut matcher = NuMatcher::new("foo bar", options);
        matcher.add("foo/bar", "foo/bar");
        matcher.add("bar/foo", "bar/foo");
        matcher.add("foobar", "foobar");
        assert_eq!(vec!["bar/foo", "foo/bar", "foobar"], matcher.get_results());
    }
}
