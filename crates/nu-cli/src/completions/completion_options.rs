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
}

pub struct NuMatcher<'a, T> {
    options: &'a CompletionOptions,
    should_sort: bool,
    needle: String,
    state: State<T>,
}

enum State<T> {
    Unscored {
        case_sensitive_needle: Option<Box<str>>,
        matches: Vec<UnscoredMatch<T>>,
    },
    Fuzzy {
        matcher: Matcher,
        atom: Atom,
        matches: Vec<FuzzyMatch<T>>,
    },
}

struct UnscoredMatch<T> {
    item: T,
    haystack: String,
    folded_haystack: Option<String>,
    match_indices: Vec<usize>,
}

struct FuzzyMatch<T> {
    item: T,
    haystack: String,
    score: u16,
    case_sensitive_match: bool,
    match_indices: Vec<usize>,
}

/// Match work that has already been computed but not yet committed to the result set.
/// Completion uses this to avoid running the matcher twice when it needs to perform an
/// additional filesystem check only for matching entries.
pub(super) struct PreparedMatch {
    haystack_start: usize,
    haystack_end: usize,
    score: Option<u16>,
    folded_haystack: Option<String>,
    match_indices: Vec<usize>,
}

const QUOTES: [char; 3] = ['"', '\'', '`'];

fn match_has_exact_case(needle: &str, haystack: &str, indices: &[usize]) -> bool {
    if needle.is_ascii() && haystack.is_ascii() {
        let needle = needle.as_bytes();
        let haystack = haystack.as_bytes();
        return needle.len() == indices.len()
            && needle
                .iter()
                .zip(indices)
                .all(|(needle, index)| haystack.get(*index) == Some(needle));
    }

    let mut needle = needle
        .graphemes(true)
        .filter_map(|grapheme| grapheme.chars().next());
    let mut haystack = haystack.graphemes(true).enumerate();
    for index in indices {
        let Some(needle) = needle.next() else {
            return false;
        };
        let Some((_, haystack)) = haystack.find(|(candidate, _)| candidate == index) else {
            return false;
        };
        if !haystack.starts_with(needle) {
            return false;
        }
    }
    needle.next().is_none()
}

#[inline(never)]
fn finish_substring_case_preferred<T>(
    matches: Vec<UnscoredMatch<T>>,
    needle: &str,
) -> Vec<(T, Vec<usize>)> {
    let has_exact_case = |mat: &UnscoredMatch<T>| {
        let exact_at_match = mat.match_indices.first().and_then(|start| {
            mat.haystack
                .as_bytes()
                .get(*start..start.saturating_add(needle.len()))
        }) == Some(needle.as_bytes());
        exact_at_match || mat.haystack.contains(needle)
    };

    let len = matches.len();
    let mut matches = matches.into_iter();
    let first = matches.next().expect("matches is not empty");
    let first_exact = has_exact_case(&first);
    let mut primary = Vec::with_capacity(len);
    primary.push((first.item, first.match_indices));
    let mut secondary = None;

    for mat in matches {
        let exact_case = has_exact_case(&mat);
        let result = (mat.item, mat.match_indices);
        if exact_case == first_exact {
            primary.push(result);
        } else {
            secondary
                .get_or_insert_with(|| Vec::with_capacity(len))
                .push(result);
        }
    }

    if let Some(mut secondary) = secondary {
        if first_exact {
            primary.extend(secondary);
            primary
        } else {
            secondary.extend(primary);
            secondary
        }
    } else {
        primary
    }
}

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
        match options.match_algorithm {
            MatchAlgorithm::Prefix | MatchAlgorithm::Substring => {
                let lowercase_needle = if options.case_sensitive {
                    needle.to_owned()
                } else if needle.is_ascii() {
                    needle.to_ascii_lowercase()
                } else {
                    needle.to_folded_case()
                };
                let case_sensitive_needle =
                    (!options.case_sensitive && should_sort && lowercase_needle != needle)
                        .then(|| needle.to_owned().into_boxed_str());
                NuMatcher {
                    options,
                    should_sort,
                    needle: lowercase_needle,
                    state: State::Unscored {
                        case_sensitive_needle,
                        matches: Vec::new(),
                    },
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
                    should_sort,
                    needle: needle.to_owned(),
                    state: State::Fuzzy {
                        matcher: Matcher::new({
                            let mut cfg = Config::DEFAULT;
                            cfg.prefer_prefix = true;
                            cfg
                        }),
                        atom,
                        matches: Vec::new(),
                    },
                }
            }
        }
    }

    /// Compute a match without committing an item to the result set yet.
    ///
    /// This lets callers perform work that should happen only for matching candidates and then
    /// commit the already-computed score/indices without running the matcher a second time.
    pub(super) fn prepare_match(&mut self, orig_haystack: &str) -> Option<PreparedMatch> {
        let haystack = orig_haystack.trim_start_matches(QUOTES);
        let haystack_start = orig_haystack.len() - haystack.len();
        let haystack = haystack.trim_end_matches(QUOTES);
        let haystack_end = haystack_start + haystack.len();

        match &mut self.state {
            State::Unscored { .. } => {
                if !self.options.case_sensitive
                    && self.options.match_algorithm == MatchAlgorithm::Prefix
                    && haystack.is_ascii()
                    && self.needle.is_ascii()
                {
                    let needle_len = self.needle.len();
                    let prefix = haystack.as_bytes().get(..needle_len)?;
                    if !prefix.eq_ignore_ascii_case(self.needle.as_bytes()) {
                        return None;
                    }
                    return Some(PreparedMatch {
                        haystack_start,
                        haystack_end,
                        score: None,
                        folded_haystack: self.should_sort.then(|| haystack.to_ascii_lowercase()),
                        match_indices: (haystack_start..haystack_start + needle_len).collect(),
                    });
                }

                let haystack_folded = if self.options.case_sensitive {
                    Cow::Borrowed(haystack)
                } else {
                    Cow::Owned(haystack.to_folded_case())
                };
                let match_start = match self.options.match_algorithm {
                    MatchAlgorithm::Prefix => {
                        if haystack_folded.starts_with(self.needle.as_str()) {
                            Some(0)
                        } else {
                            None
                        }
                    }
                    MatchAlgorithm::Substring => haystack_folded.find(self.needle.as_str()),
                    _ => unreachable!("Only prefix and substring algorithms don't use score"),
                };
                let byte_start = match_start?;
                let grapheme_start = haystack_folded[0..byte_start].graphemes(true).count();
                // TODO this doesn't account for lowercasing changing the length of the haystack
                let grapheme_len = self.needle.graphemes(true).count();
                let folded_haystack = if self.should_sort {
                    match haystack_folded {
                        Cow::Owned(folded) => Some(folded),
                        Cow::Borrowed(_) => None,
                    }
                } else {
                    None
                };
                Some(PreparedMatch {
                    haystack_start,
                    haystack_end,
                    score: None,
                    folded_haystack,
                    match_indices: (haystack_start + grapheme_start
                        ..haystack_start + grapheme_start + grapheme_len)
                        .collect(),
                })
            }
            State::Fuzzy { matcher, atom, .. } => {
                let mut haystack_buf = Vec::new();
                let haystack_utf32 = Utf32Str::new(haystack, &mut haystack_buf);
                let mut indices = Vec::new();
                let score = atom.indices(haystack_utf32, matcher, &mut indices)?;
                Some(PreparedMatch {
                    haystack_start,
                    haystack_end,
                    score: Some(score),
                    folded_haystack: None,
                    match_indices: indices
                        .iter()
                        .map(|i| {
                            haystack_start
                                + usize::try_from(*i)
                                    .expect("should be on at least a 32-bit system")
                        })
                        .collect(),
                })
            }
        }
    }

    /// Commit an already-computed match, reusing the caller's owned haystack allocation.
    pub(super) fn add_prepared_owned(
        &mut self,
        mut haystack: String,
        prepared: PreparedMatch,
        item: T,
    ) {
        let fuzzy_case_sensitive_match = !self.options.case_sensitive
            && self.should_sort
            && self.options.match_algorithm == MatchAlgorithm::Fuzzy
            && match_has_exact_case(self.needle.as_str(), &haystack, &prepared.match_indices);

        haystack.truncate(prepared.haystack_end);
        if prepared.haystack_start != 0 {
            haystack.replace_range(..prepared.haystack_start, "");
        }

        match &mut self.state {
            State::Unscored { matches, .. } => {
                debug_assert!(prepared.score.is_none());
                matches.push(UnscoredMatch {
                    item,
                    haystack,
                    folded_haystack: prepared.folded_haystack,
                    match_indices: prepared.match_indices,
                });
            }
            State::Fuzzy { matches, .. } => {
                matches.push(FuzzyMatch {
                    item,
                    haystack,
                    score: prepared
                        .score
                        .expect("prepared fuzzy match should contain a score"),
                    case_sensitive_match: fuzzy_case_sensitive_match,
                    match_indices: prepared.match_indices,
                });
            }
        }
    }

    /// Add the given item if the given haystack matches the needle.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        let haystack = haystack.as_ref();
        let Some(prepared) = self.prepare_match(haystack) else {
            return false;
        };
        self.add_prepared_owned(haystack.to_owned(), prepared, item);
        true
    }

    /// Check if the given haystack matches the needle without adding it as a result.
    ///
    /// Returns match indices if it matched, None if it didn't.
    pub fn check_match(&mut self, haystack: &str) -> Option<Vec<usize>> {
        self.prepare_match(haystack)
            .map(|prepared| prepared.match_indices)
    }

    fn sort(&mut self) {
        match &mut self.state {
            State::Unscored { matches, .. } => {
                matches.sort_by(|a, b| {
                    let cmp_sensitive = a.haystack.cmp(&b.haystack);
                    if self.options.case_sensitive {
                        cmp_sensitive
                    } else {
                        a.folded_haystack
                            .as_deref()
                            .expect("case-insensitive match should retain folded haystack")
                            .cmp(
                                b.folded_haystack
                                    .as_deref()
                                    .expect("case-insensitive match should retain folded haystack"),
                            )
                            .then(cmp_sensitive)
                    }
                });
            }
            State::Fuzzy { matches, .. } => match self.options.sort {
                CompletionSort::Alphabetical => {
                    matches.sort_by(|a, b| {
                        b.case_sensitive_match
                            .cmp(&a.case_sensitive_match)
                            .then_with(|| a.haystack.cmp(&b.haystack))
                    });
                }
                CompletionSort::Smart => {
                    matches.sort_by(|a, b| {
                        b.case_sensitive_match
                            .cmp(&a.case_sensitive_match)
                            .then_with(|| b.score.cmp(&a.score))
                            .then_with(|| a.haystack.cmp(&b.haystack))
                    });
                }
            },
        }
    }

    /// Sort and return all the matches, along with their match indices
    pub fn results(mut self) -> Vec<(T, Vec<usize>)> {
        if self.should_sort {
            self.sort();
        }
        match self.state {
            State::Unscored {
                case_sensitive_needle,
                matches,
            } => {
                if !self.options.case_sensitive && self.should_sort && !matches.is_empty() {
                    let needle = case_sensitive_needle
                        .as_deref()
                        .unwrap_or(self.needle.as_str());
                    match self.options.match_algorithm {
                        MatchAlgorithm::Prefix => {
                            let len = matches.len();
                            let mut matches = matches.into_iter();
                            let first = matches.next().expect("matches is not empty");
                            let first_exact = first.haystack.starts_with(needle);
                            let mut primary = Vec::with_capacity(len);
                            primary.push((first.item, first.match_indices));
                            let mut secondary = None;

                            for mat in matches {
                                let exact_case = mat.haystack.starts_with(needle);
                                let result = (mat.item, mat.match_indices);
                                if exact_case == first_exact {
                                    primary.push(result);
                                } else {
                                    secondary
                                        .get_or_insert_with(|| Vec::with_capacity(len))
                                        .push(result);
                                }
                            }

                            if let Some(mut secondary) = secondary {
                                if first_exact {
                                    primary.extend(secondary);
                                    primary
                                } else {
                                    secondary.extend(primary);
                                    secondary
                                }
                            } else {
                                primary
                            }
                        }
                        MatchAlgorithm::Substring => {
                            finish_substring_case_preferred(matches, needle)
                        }
                        MatchAlgorithm::Fuzzy => unreachable!("fuzzy matches use scored state"),
                    }
                } else {
                    matches
                        .into_iter()
                        .map(|mat| (mat.item, mat.match_indices))
                        .collect()
                }
            }
            State::Fuzzy { matches, .. } => matches
                .into_iter()
                .map(|mat| (mat.item, mat.match_indices))
                .collect(),
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

    use super::{CompletionOptions, CompletionSort, MatchAlgorithm, NuMatcher};

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
        let mut matcher = NuMatcher::new(needle, &options, true);
        matcher.add(haystack, haystack);
        let results: Vec<_> = matcher.results().iter().map(|r| r.0).collect();
        if should_match {
            assert_eq!(vec![haystack], results);
        } else {
            assert_ne!(vec![haystack], results);
        }
    }

    #[rstest]
    #[case(MatchAlgorithm::Prefix, CompletionSort::Smart)]
    #[case(MatchAlgorithm::Substring, CompletionSort::Smart)]
    #[case(MatchAlgorithm::Fuzzy, CompletionSort::Smart)]
    #[case(MatchAlgorithm::Fuzzy, CompletionSort::Alphabetical)]
    fn case_insensitive_sort_prefers_case_sensitive_match(
        #[case] match_algorithm: MatchAlgorithm,
        #[case] sort: CompletionSort,
    ) {
        let options = CompletionOptions {
            case_sensitive: false,
            match_algorithm,
            sort,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("test-t", &options, true);
        for item in ["test-Test", "test-test"] {
            matcher.add(item, item);
        }

        let results: Vec<_> = matcher
            .results()
            .into_iter()
            .map(|result| result.0)
            .collect();
        assert_eq!(vec!["test-test", "test-Test"], results);
    }

    #[rstest]
    #[case(MatchAlgorithm::Prefix)]
    #[case(MatchAlgorithm::Substring)]
    #[case(MatchAlgorithm::Fuzzy)]
    fn case_sensitive_match_precedes_better_alphabetical_insensitive_match(
        #[case] match_algorithm: MatchAlgorithm,
    ) {
        let options = CompletionOptions {
            case_sensitive: false,
            match_algorithm,
            sort: CompletionSort::Smart,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("test-t", &options, true);
        for item in ["test-Ta", "test-tz"] {
            matcher.add(item, item);
        }

        let results: Vec<_> = matcher
            .results()
            .into_iter()
            .map(|result| result.0)
            .collect();
        assert_eq!(vec!["test-tz", "test-Ta"], results);
    }

    #[test]
    fn case_preference_does_not_reorder_unsorted_matches() {
        let options = CompletionOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new("test-t", &options, false);
        for item in ["test-Test", "test-test"] {
            matcher.add(item, item);
        }

        let results: Vec<_> = matcher
            .results()
            .into_iter()
            .map(|result| result.0)
            .collect();
        assert_eq!(vec!["test-Test", "test-test"], results);
    }

    #[rstest]
    #[case(MatchAlgorithm::Prefix)]
    #[case(MatchAlgorithm::Substring)]
    #[case(MatchAlgorithm::Fuzzy)]
    fn prepared_match_is_equivalent_to_add(#[case] match_algorithm: MatchAlgorithm) {
        let options = CompletionOptions {
            case_sensitive: false,
            match_algorithm,
            ..Default::default()
        };
        let haystacks = ["foo", "'Foo bar'", "food", "bar", "`fob`", "afoo"];
        let mut normal = NuMatcher::new("foo", &options, true);
        let mut prepared = NuMatcher::new("foo", &options, true);

        for (index, haystack) in haystacks.into_iter().enumerate() {
            normal.add(haystack, index);
            if let Some(matched) = prepared.prepare_match(haystack) {
                prepared.add_prepared_owned(haystack.to_owned(), matched, index);
            }
        }

        assert_eq!(normal.results(), prepared.results());
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
