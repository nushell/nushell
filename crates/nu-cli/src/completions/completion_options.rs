use nu_protocol::{CompletionAlgorithm, CompletionSort};
use nu_utils::IgnoreCaseExt;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};
use std::{borrow::Cow, fmt::Display};
use unicode_segmentation::UnicodeSegmentation;

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

    /// Shows prefix matches when any exist; otherwise falls back to fuzzy matches
    ///
    /// Examples:
    /// - "git sw" matches "git switch" by prefix
    /// - "gco" matches "git checkout" by fuzzy matching when no prefix matches exist
    Fallback,
}

pub struct NuMatcher<'a, T> {
    options: &'a CompletionOptions,
    should_sort: bool,
    needle: String,
    state: State<T>,
}

enum State<T> {
    Unscored(Vec<UnscoredMatch<T>>),
    Fuzzy {
        matcher: Matcher,
        atom: Atom,
        matches: Vec<FuzzyMatch<T>>,
    },
    Fallback {
        matcher: Matcher,
        atom: Atom,
        prefix_matches: Vec<UnscoredMatch<T>>,
        fuzzy_matches: Vec<FuzzyMatch<T>>,
    },
}

struct UnscoredMatch<T> {
    item: T,
    haystack: String,
    match_indices: Vec<usize>,
}

struct FuzzyMatch<T> {
    item: T,
    haystack: String,
    score: u16,
    match_indices: Vec<usize>,
}

const QUOTES: [char; 3] = ['"', '\'', '`'];

/// Filters and sorts suggestions
impl<T> NuMatcher<'_, T> {
    /// # Arguments
    ///
    /// * `needle` - The text to search for
    /// * `should_sort` - Should results be sorted?
    pub fn new(
        needle: impl AsRef<str>,
        options: &CompletionOptions,
        should_sort: bool,
    ) -> NuMatcher<'_, T> {
        // NOTE: Should match `'bar baz'` when completing `foo "b<tab>`
        // https://github.com/nushell/nushell/issues/16860#issuecomment-3402016955
        let needle = needle.as_ref().trim_matches(QUOTES);
        let make_atom = || {
            Atom::new(
                needle,
                if options.case_sensitive {
                    CaseMatching::Respect
                } else {
                    CaseMatching::Ignore
                },
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            )
        };
        let make_matcher = || {
            Matcher::new({
                let mut cfg = Config::DEFAULT;
                cfg.prefer_prefix = true;
                cfg
            })
        };
        match options.match_algorithm {
            MatchAlgorithm::Prefix | MatchAlgorithm::Substring => {
                let needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_folded_case()
                };
                NuMatcher {
                    options,
                    should_sort,
                    needle,
                    state: State::Unscored(Vec::new()),
                }
            }
            MatchAlgorithm::Fuzzy => NuMatcher {
                options,
                should_sort,
                needle: needle.to_owned(),
                state: State::Fuzzy {
                    matcher: make_matcher(),
                    atom: make_atom(),
                    matches: Vec::new(),
                },
            },
            MatchAlgorithm::Fallback => {
                let needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_folded_case()
                };
                NuMatcher {
                    options,
                    should_sort,
                    needle,
                    state: State::Fallback {
                        matcher: make_matcher(),
                        atom: make_atom(),
                        prefix_matches: Vec::new(),
                        fuzzy_matches: Vec::new(),
                    },
                }
            }
        }
    }

    fn unscored_matches(
        haystack: &str,
        needle: &str,
        match_algorithm: MatchAlgorithm,
        offset: usize,
        case_sensitive: bool,
    ) -> Option<Vec<usize>> {
        let haystack_folded = if case_sensitive {
            Cow::Borrowed(haystack)
        } else {
            Cow::Owned(haystack.to_folded_case())
        };
        let match_start = match match_algorithm {
            MatchAlgorithm::Prefix => {
                if haystack_folded.starts_with(needle) {
                    Some(0)
                } else {
                    None
                }
            }
            MatchAlgorithm::Substring => haystack_folded.find(needle),
            MatchAlgorithm::Fuzzy | MatchAlgorithm::Fallback => None,
        };
        match_start.map(|byte_start| {
            let grapheme_start = haystack_folded[0..byte_start].graphemes(true).count();
            // TODO this doesn't account for lowercasing changing the length of the haystack
            let grapheme_len = needle.graphemes(true).count();
            (offset + grapheme_start..offset + grapheme_start + grapheme_len).collect()
        })
    }

    fn fuzzy_matches(
        haystack: &str,
        atom: &Atom,
        matcher: &mut Matcher,
        offset: usize,
    ) -> Option<(Vec<usize>, u16)> {
        let mut haystack_buf = Vec::new();
        let haystack_utf32 = Utf32Str::new(haystack, &mut haystack_buf);
        let mut indices = Vec::new();
        let score = atom.indices(haystack_utf32, matcher, &mut indices)?;
        Some((
            indices
                .iter()
                .map(|i| {
                    offset + usize::try_from(*i).expect("should be on at least a 32-bit system")
                })
                .collect(),
            score,
        ))
    }

    /// Returns whether or not the haystack matches the needle. If it does, `item` is added
    /// to the list of matches (if given).
    ///
    /// Helper to avoid code duplication between [NuMatcher::add] and [NuMatcher::matches].
    fn matches_aux(&mut self, orig_haystack: &str, item: Option<T>) -> Option<Vec<usize>> {
        let haystack = orig_haystack.trim_start_matches(QUOTES);
        let offset = orig_haystack.len() - haystack.len();
        let haystack = haystack.trim_end_matches(QUOTES);
        match &mut self.state {
            State::Unscored(matches) => {
                let match_indices = Self::unscored_matches(
                    haystack,
                    self.needle.as_str(),
                    self.options.match_algorithm,
                    offset,
                    self.options.case_sensitive,
                )?;
                if let Some(item) = item {
                    matches.push(UnscoredMatch {
                        item,
                        haystack: haystack.to_string(),
                        match_indices: match_indices.clone(),
                    });
                }
                Some(match_indices)
            }
            State::Fuzzy {
                matcher,
                atom,
                matches,
            } => {
                let (match_indices, score) = Self::fuzzy_matches(haystack, atom, matcher, offset)?;
                if let Some(item) = item {
                    matches.push(FuzzyMatch {
                        item,
                        haystack: haystack.to_string(),
                        score,
                        match_indices: match_indices.clone(),
                    });
                }
                Some(match_indices)
            }
            State::Fallback {
                matcher,
                atom,
                prefix_matches,
                fuzzy_matches,
            } => {
                // prefix
                if let Some(match_indices) = Self::unscored_matches(
                    haystack,
                    self.needle.as_str(),
                    MatchAlgorithm::Prefix,
                    offset,
                    self.options.case_sensitive,
                ) {
                    if let Some(item) = item {
                        prefix_matches.push(UnscoredMatch {
                            item,
                            haystack: haystack.to_string(),
                            match_indices: match_indices.clone(),
                        });
                    }
                    return Some(match_indices);
                }

                // fallback to fuzzy
                let (match_indices, score) = Self::fuzzy_matches(haystack, atom, matcher, offset)?;
                if let Some(item) = item {
                    fuzzy_matches.push(FuzzyMatch {
                        item,
                        haystack: haystack.to_string(),
                        score,
                        match_indices: match_indices.clone(),
                    });
                }
                Some(match_indices)
            }
        }
    }

    /// Add the given item if the given haystack matches the needle.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        self.matches_aux(haystack.as_ref(), Some(item)).is_some()
    }

    /// Returns whether the haystack matches the needle using prefix matching.
    pub(crate) fn has_prefix_match(&self, orig_haystack: &str) -> bool {
        let haystack = orig_haystack.trim_start_matches(QUOTES);
        let offset = orig_haystack.len() - haystack.len();
        let haystack = haystack.trim_end_matches(QUOTES);
        Self::unscored_matches(
            haystack,
            self.needle.as_str(),
            MatchAlgorithm::Prefix,
            offset,
            self.options.case_sensitive,
        )
        .is_some()
    }

    /// Check if the given haystack matches the needle without adding it as a result.
    ///
    /// Returns match indices if it matched, None if it didn't.
    pub fn check_match(&mut self, haystack: &str) -> Option<Vec<usize>> {
        self.matches_aux(haystack, None)
    }

    fn sort(&mut self) {
        let sort_unscored = |matches: &mut [UnscoredMatch<T>]| {
            matches.sort_by(|a, b| {
                let cmp_sensitive = a.haystack.cmp(&b.haystack);
                if self.options.case_sensitive {
                    cmp_sensitive
                } else {
                    a.haystack
                        .to_folded_case()
                        .cmp(&b.haystack.to_folded_case())
                        .then(cmp_sensitive)
                }
            });
        };
        let sort_fuzzy = |matches: &mut [FuzzyMatch<T>]| match self.options.sort {
            CompletionSort::Alphabetical => {
                matches.sort_by(|a, b| a.haystack.cmp(&b.haystack));
            }
            CompletionSort::Smart => {
                matches.sort_by(|a, b| b.score.cmp(&a.score).then(a.haystack.cmp(&b.haystack)));
            }
        };
        match &mut self.state {
            State::Unscored(matches) => sort_unscored(matches),
            State::Fuzzy { matches, .. } => sort_fuzzy(matches),
            State::Fallback {
                prefix_matches,
                fuzzy_matches,
                ..
            } => {
                if prefix_matches.is_empty() {
                    sort_fuzzy(fuzzy_matches);
                } else {
                    sort_unscored(prefix_matches);
                }
            }
        }
    }

    /// Sort and return all the matches, along with their match indices
    pub fn results(mut self) -> Vec<(T, Vec<usize>)> {
        if self.should_sort {
            self.sort();
        }
        match self.state {
            State::Unscored(matches) => matches
                .into_iter()
                .map(|mat| (mat.item, mat.match_indices))
                .collect::<Vec<_>>(),
            State::Fuzzy { matches, .. } => matches
                .into_iter()
                .map(|mat| (mat.item, mat.match_indices))
                .collect::<Vec<_>>(),
            State::Fallback {
                prefix_matches,
                fuzzy_matches,
                ..
            } => {
                if prefix_matches.is_empty() {
                    fuzzy_matches
                        .into_iter()
                        .map(|mat| (mat.item, mat.match_indices))
                        .collect()
                } else {
                    prefix_matches
                        .into_iter()
                        .map(|mat| (mat.item, mat.match_indices))
                        .collect()
                }
            }
        }
    }
}

impl NuMatcher<'_, SemanticSuggestion> {
    pub fn add_semantic_suggestion(&mut self, sugg: SemanticSuggestion) -> bool {
        let value = sugg.suggestion.display_value().to_string();
        self.add(value, sugg)
    }

    /// Get all the items that matched (sorted)
    pub fn suggestion_results(self) -> Vec<SemanticSuggestion> {
        self.results()
            .into_iter()
            .map(|(mut sugg, indices)| {
                sugg.suggestion.match_indices = Some(indices);
                sugg
            })
            .collect()
    }
}

impl From<CompletionAlgorithm> for MatchAlgorithm {
    fn from(value: CompletionAlgorithm) -> Self {
        match value {
            CompletionAlgorithm::Prefix => MatchAlgorithm::Prefix,
            CompletionAlgorithm::Substring => MatchAlgorithm::Substring,
            CompletionAlgorithm::Fuzzy => MatchAlgorithm::Fuzzy,
            CompletionAlgorithm::Fallback => MatchAlgorithm::Fallback,
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
            "fallback" => Ok(Self::Fallback),
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
    pub match_description: bool,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            match_algorithm: MatchAlgorithm::Prefix,
            sort: Default::default(),
            match_description: false,
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use nu_protocol::CompletionSort;

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
    #[case(MatchAlgorithm::Fallback, "example text", "", true)]
    #[case(MatchAlgorithm::Fallback, "example text", "examp", true)]
    #[case(MatchAlgorithm::Fallback, "example text", "text", true)]
    #[case(MatchAlgorithm::Fallback, "example text", "ext", true)]
    #[case(MatchAlgorithm::Fallback, "example text", "mplxt", true)]
    #[case(MatchAlgorithm::Fallback, "example text", "mpp", false)]
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
        let mut matcher = NuMatcher::new(needle, &options, true);
        matcher.add(haystack, haystack);
        let results: Vec<_> = matcher.results().iter().map(|r| r.0).collect();
        if should_match {
            assert_eq!(vec![haystack], results);
        } else {
            assert_ne!(vec![haystack], results);
        }
    }

    #[test]
    fn fallback_prefers_and_sorts_prefix_matches() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fallback,
            sort: CompletionSort::Smart,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("foo", &options, true);
        matcher.add("barfoo", "barfoo");
        matcher.add("fooz", "fooz");
        matcher.add("fooa", "fooa");

        assert_eq!(
            vec![("fooa", vec![0, 1, 2]), ("fooz", vec![0, 1, 2])],
            matcher.results()
        );
    }

    #[test]
    fn fallback_uses_fuzzy_matches_without_prefix_matches() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fallback,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("fob", &options, true);
        matcher.add("foo/bar", "foo/bar");
        matcher.add("foo bar", "foo bar");

        assert_eq!(
            vec![("foo bar", vec![0, 1, 4]), ("foo/bar", vec![0, 1, 4]),],
            matcher.results()
        );
    }

    #[test]
    fn fallback_matches_prefixes_case_insensitively() {
        let options = CompletionOptions {
            case_sensitive: false,
            match_algorithm: MatchAlgorithm::Fallback,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("FOO", &options, true);
        matcher.add("barfoo", "barfoo");
        matcher.add("FooBar", "FooBar");

        assert_eq!(vec![("FooBar", vec![0, 1, 2])], matcher.results());
    }

    #[rstest]
    #[case::smart(
        CompletionSort::Smart,
        vec!["z fob", "foo bar", "foo/bar"]
    )]
    #[case::alphabetical(
        CompletionSort::Alphabetical,
        vec!["foo bar", "foo/bar", "z fob"]
    )]
    fn fallback_sorts_fuzzy_matches(#[case] sort: CompletionSort, #[case] expected: Vec<&str>) {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fallback,
            sort,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("fob", &options, true);
        for candidate in ["foo/bar", "z fob", "foo bar"] {
            matcher.add(candidate, candidate);
        }

        assert_eq!(
            expected,
            matcher
                .results()
                .into_iter()
                .map(|(candidate, _)| candidate)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn match_algorithm_fuzzy_sort_score() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fuzzy,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("fob", &options, true);
        for item in ["foo/bar", "fob", "foo bar"] {
            matcher.add(item, item);
        }
        // Sort by score, then in alphabetical order
        assert_eq!(
            vec![
                ("fob", vec![0, 1, 2]),
                ("foo bar", vec![0, 1, 4]),
                ("foo/bar", vec![0, 1, 4])
            ],
            matcher.results()
        );
    }

    #[test]
    fn match_algorithm_fuzzy_sort_strip() {
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fuzzy,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("'love spaces' ", &options, true);
        for item in [
            "'i love spaces'",
            "'i love spaces' so much",
            "'lovespaces' ",
        ] {
            matcher.add(item, item);
        }
        // Make sure the spaces are respected
        assert_eq!(
            vec![(
                "'i love spaces' so much",
                vec![3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
            )],
            matcher.results()
        );
    }
}
