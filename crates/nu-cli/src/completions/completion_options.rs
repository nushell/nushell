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
    Unscored(Vec<UnscoredMatch<T>>),
    Fuzzy {
        matcher: Matcher,
        atom: Atom,
        matches: Vec<FuzzyMatch<T>>,
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
        match options.match_algorithm {
            MatchAlgorithm::Prefix | MatchAlgorithm::Substring => {
                let lowercase_needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_folded_case()
                };
                NuMatcher {
                    options,
                    should_sort,
                    needle: lowercase_needle,
                    state: State::Unscored(Vec::new()),
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

    /// Returns whether or not the haystack matches the needle. If it does, `item` is added
    /// to the list of matches (if given).
    ///
    /// Helper to avoid code duplication between [NuMatcher::add] and [NuMatcher::matches].
    fn matches_aux(&mut self, orig_haystack: &str, item: Option<T>) -> Option<Vec<usize>> {
        let haystack = orig_haystack.trim_start_matches(QUOTES);
        let offset = orig_haystack.len() - haystack.len();
        // A combining mark can share a displayed grapheme with a stripped quote.
        let offset = if offset == 0 || orig_haystack.is_ascii() {
            offset
        } else {
            orig_haystack
                .grapheme_indices(true)
                .take_while(|(start, _)| *start <= offset)
                .count()
                .saturating_sub(1)
        };
        let haystack = haystack.trim_end_matches(QUOTES);
        match &mut self.state {
            State::Unscored(matches) => {
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
                match_start.map(|byte_start| {
                    let byte_end = byte_start + self.needle.len();
                    let match_indices: Vec<usize> =
                        if haystack.is_ascii() && !haystack.contains('\r') {
                            (offset + byte_start..offset + byte_end).collect()
                        } else {
                            // Folding can expand a displayed grapheme (`ß` becomes `ss`). Map
                            // overlapping folded byte ranges back to the original graphemes.
                            let mut indices = Vec::new();
                            let mut end = 0;
                            for (index, grapheme) in haystack.graphemes(true).enumerate() {
                                if end >= byte_end {
                                    break;
                                }
                                // Reuse the folded text when the byte length is unchanged,
                                // including expansions such as `ß` -> `ss`. Only folds that
                                // change byte length need another allocation (`İ` -> `i̇`).
                                let same_folded_byte_len = self.options.case_sensitive
                                    || grapheme.is_ascii()
                                    || haystack_folded
                                        .get(end..end + grapheme.len())
                                        .is_some_and(|folded| grapheme.eq_ignore_case(folded));
                                end += if same_folded_byte_len {
                                    grapheme.len()
                                } else {
                                    grapheme.to_folded_case().len()
                                };
                                if byte_start < end {
                                    indices.push(offset + index);
                                }
                            }
                            indices
                        };
                    if let Some(item) = item {
                        matches.push(UnscoredMatch {
                            item,
                            haystack: haystack.to_string(),
                            match_indices: match_indices.clone(),
                        });
                    }
                    match_indices
                })
            }
            State::Fuzzy {
                matcher,
                atom,
                matches,
            } => {
                let mut haystack_buf = Vec::new();
                // nucleo 0.3's constructor can incorrectly reuse UTF-8 bytes when every
                // grapheme starts with ASCII, such as `e` followed by a combining accent.
                // CRLF is also a single grapheme, despite containing two ASCII bytes.
                let haystack_utf32 = if haystack.is_ascii() && !haystack.contains('\r') {
                    Utf32Str::Ascii(haystack.as_bytes())
                } else {
                    haystack_buf.extend(nucleo_matcher::chars::graphemes(haystack));
                    Utf32Str::Unicode(&haystack_buf)
                };
                let mut indices = Vec::new();
                let score = atom.indices(haystack_utf32, matcher, &mut indices)?;
                let indices: Vec<usize> = indices
                    .iter()
                    .map(|i| {
                        offset + usize::try_from(*i).expect("should be on at least a 32-bit system")
                    })
                    .collect();
                if let Some(item) = item {
                    matches.push(FuzzyMatch {
                        item,
                        haystack: haystack.to_string(),
                        score,
                        match_indices: indices.clone(),
                    });
                }
                Some(indices)
            }
        }
    }

    /// Add the given item if the given haystack matches the needle.
    ///
    /// Returns whether the item was added.
    pub fn add(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        self.matches_aux(haystack.as_ref(), Some(item)).is_some()
    }

    /// Check if the given haystack matches the needle without adding it as a result.
    ///
    /// Returns match indices if it matched, None if it didn't.
    pub fn check_match(&mut self, haystack: &str) -> Option<Vec<usize>> {
        self.matches_aux(haystack, None)
    }

    fn sort(&mut self) {
        match &mut self.state {
            State::Unscored(matches) => {
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
            }
            State::Fuzzy { matches, .. } => match self.options.sort {
                CompletionSort::Alphabetical => {
                    matches.sort_by(|a, b| a.haystack.cmp(&b.haystack));
                }
                CompletionSort::Smart => {
                    matches.sort_by(|a, b| b.score.cmp(&a.score).then(a.haystack.cmp(&b.haystack)));
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
            State::Unscored(matches) => matches
                .into_iter()
                .map(|mat| (mat.item, mat.match_indices))
                .collect::<Vec<_>>(),
            State::Fuzzy { matches, .. } => matches
                .into_iter()
                .map(|mat| (mat.item, mat.match_indices))
                .collect::<Vec<_>>(),
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

    #[rstest]
    #[case(MatchAlgorithm::Prefix, false, "STRASSE", "Straße", vec![0, 1, 2, 3, 4, 5])]
    #[case(MatchAlgorithm::Prefix, false, "ss", "ß", vec![0])]
    #[case(MatchAlgorithm::Substring, false, "e", "Straße", vec![5])]
    #[case(MatchAlgorithm::Substring, false, "x", "ßx", vec![1])]
    #[case(MatchAlgorithm::Substring, false, "s", "ßx", vec![0])]
    #[case(MatchAlgorithm::Substring, false, "ss", "\"ßx\"", vec![1])]
    #[case(MatchAlgorithm::Substring, true, "\u{301}", "e\u{301}x", vec![0])]
    #[case(MatchAlgorithm::Substring, true, "\u{301}", "\"\u{301}x\"", vec![0])]
    #[case(MatchAlgorithm::Substring, true, "x", "\"\u{301}x\"", vec![1])]
    #[case(MatchAlgorithm::Prefix, false, "i", "İx", vec![0])]
    #[case(MatchAlgorithm::Substring, false, "x", "İx", vec![1])]
    #[case(MatchAlgorithm::Substring, false, "", "ßx", vec![])]
    #[case(MatchAlgorithm::Fuzzy, false, "z", "e\u{301}z", vec![1])]
    #[case(MatchAlgorithm::Fuzzy, true, "z", "\"e\u{301}z\"", vec![2])]
    #[case(MatchAlgorithm::Fuzzy, true, "x", "\"\u{301}x\"", vec![1])]
    #[case(MatchAlgorithm::Substring, true, "b", "a\r\nb", vec![2])]
    #[case(MatchAlgorithm::Substring, false, "B", "\"a\r\nb\"", vec![3])]
    #[case(MatchAlgorithm::Prefix, true, "a\r\nb", "a\r\nb", vec![0, 1, 2])]
    #[case(MatchAlgorithm::Prefix, false, "A\r\nB", "'a\r\nb'", vec![1, 2, 3])]
    #[case(MatchAlgorithm::Fuzzy, false, "b", "a\r\nb", vec![2])]
    #[case(MatchAlgorithm::Fuzzy, true, "b", "\"a\r\nb\"", vec![3])]
    #[case(MatchAlgorithm::Substring, false, "ffi", "aﬃb", vec![1])]
    #[case(MatchAlgorithm::Substring, false, "b", "aﬃb", vec![2])]
    #[case(MatchAlgorithm::Substring, false, "x", "Kx", vec![1])]
    #[case(MatchAlgorithm::Prefix, false, "k", "Kx", vec![0])]
    #[case(MatchAlgorithm::Substring, false, "x", "ΐx", vec![1])]
    #[case(MatchAlgorithm::Prefix, false, "ι", "ΐx", vec![0])]
    fn match_indices_follow_original_graphemes(
        #[case] match_algorithm: MatchAlgorithm,
        #[case] case_sensitive: bool,
        #[case] needle: &str,
        #[case] haystack: &str,
        #[case] expected: Vec<usize>,
    ) {
        let options = CompletionOptions {
            match_algorithm,
            case_sensitive,
            ..Default::default()
        };
        let mut matcher = NuMatcher::new(needle, &options, true);
        assert_eq!(matcher.check_match(haystack), Some(expected.clone()));
        assert!(matcher.add(haystack, haystack));
        assert_eq!(matcher.results(), vec![(haystack, expected)]);
    }
}
