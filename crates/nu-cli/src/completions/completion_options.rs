use nu_parser::trim_quotes_str;
use nu_protocol::CompletionAlgorithm;
use nu_utils::IgnoreCaseExt;
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};
use std::{borrow::Cow, fmt::Display};

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

impl MatchAlgorithm {
    // / Keeps only items that match the given `needle`
    // /
    // / # Arguments
    // /
    // / * `items` - A list of haystacks and their corresponding items
    // / * `needle` - The text to search for
    // / * `case_sensitive` - true to respect case, false to ignore it
    // /
    // / # Returns
    // /
    // / A list of matching items, as well as the indices in their haystacks that matched
}

pub struct NuMatcher<T> {
    state: State<T>,
}

enum State<T> {
    Prefix {
        needle: Vec<u8>,
        case_sensitive: bool,
        items: Vec<T>,
    },
    Nucleo {
        matcher: Matcher,
        pat: Pattern,
        items: Vec<(u32, T, Vec<usize>)>,
    },
}

impl<T> NuMatcher<T> {
    pub fn from_str(
        options: &CompletionOptions,
        needle: impl AsRef<str>,
        match_paths: bool,
    ) -> NuMatcher<T> {
        let needle = trim_quotes_str(needle.as_ref());
        match options.match_algorithm {
            MatchAlgorithm::Prefix => {
                let needle = if options.case_sensitive {
                    Cow::Borrowed(needle)
                } else {
                    Cow::Owned(needle.to_folded_case())
                };
                NuMatcher {
                    state: State::Prefix {
                        needle: needle.as_bytes().to_vec(),
                        case_sensitive: options.case_sensitive,
                        items: Vec::new(),
                    },
                }
            }
            MatchAlgorithm::Fuzzy => {
                let matcher = Matcher::new(if match_paths {
                    Config::DEFAULT.match_paths()
                } else {
                    Config::DEFAULT
                });
                let pat = Pattern::new(
                    needle,
                    if options.case_sensitive {
                        CaseMatching::Respect
                    } else {
                        CaseMatching::Ignore
                    },
                    Normalization::Smart,
                    AtomKind::Fuzzy,
                );
                NuMatcher {
                    state: State::Nucleo {
                        matcher,
                        pat,
                        items: Vec::new(),
                    },
                }
            }
        }
    }

    pub fn from_u8(
        options: &CompletionOptions,
        needle: impl AsRef<[u8]>,
        match_paths: bool,
    ) -> NuMatcher<T> {
        let needle = needle.as_ref();
        match options.match_algorithm {
            MatchAlgorithm::Prefix => {
                let needle = if options.case_sensitive {
                    needle.to_owned()
                } else {
                    needle.to_ascii_lowercase()
                };
                NuMatcher {
                    state: State::Prefix {
                        needle,
                        case_sensitive: options.case_sensitive,
                        items: Vec::new(),
                    },
                }
            }
            MatchAlgorithm::Fuzzy => {
                Self::from_str(options, String::from_utf8_lossy(needle), match_paths)
            }
        }
    }

    pub fn add_str(&mut self, haystack: impl AsRef<str>, item: T) -> bool {
        match &mut self.state {
            State::Prefix {
                needle,
                case_sensitive,
                items,
            } => {
                let haystack = trim_quotes_str(haystack.as_ref());
                let haystack = if *case_sensitive {
                    Cow::Borrowed(haystack)
                } else {
                    Cow::Owned(haystack.to_folded_case())
                };
                if haystack.as_bytes().starts_with(needle) {
                    items.push(item);
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
                let haystack = Utf32Str::new(trim_quotes_str(haystack.as_ref()), &mut haystack_buf);
                // todo find out why nucleo uses u32 instead of usize for indices
                let mut indices = Vec::new();
                match pat.indices(haystack, matcher, &mut indices) {
                    Some(score) => {
                        indices.sort_unstable();
                        indices.dedup();
                        items.push((
                            score,
                            item,
                            indices.into_iter().map(|i| i as usize).collect(),
                        ));
                        true
                    }
                    None => false,
                }
            }
        }
    }

    pub fn add_u8(&mut self, haystack: impl AsRef<[u8]>, item: T) -> bool {
        let haystack = haystack.as_ref();
        match &mut self.state {
            State::Prefix {
                needle,
                case_sensitive,
                items,
            } => {
                let haystack = if *case_sensitive {
                    Cow::Borrowed(haystack)
                } else {
                    Cow::Owned(haystack.to_ascii_lowercase())
                };
                if haystack.starts_with(needle) {
                    items.push(item);
                    true
                } else {
                    false
                }
            }
            State::Nucleo { .. } => self.add_str(String::from_utf8_lossy(haystack), item),
        }
    }

    #[allow(dead_code)]
    pub fn sort_by(&mut self, _sort_by: SortBy) {
        todo!()
    }

    pub fn get_results(self) -> Vec<T> {
        match self.state {
            State::Prefix { items, .. } => items,
            State::Nucleo { .. } => {
                let (results, _): (Vec<_>, Vec<_>) =
                    self.get_results_with_inds().into_iter().unzip();
                results
            }
        }
    }

    pub fn get_results_with_inds(self) -> Vec<(T, Vec<usize>)> {
        match self.state {
            State::Prefix { needle, items, .. } => items
                .into_iter()
                .map(|item| (item, (0..needle.len()).collect()))
                .collect(),
            State::Nucleo { items, .. } => items
                .into_iter()
                .map(|(_, items, indices)| (items, indices))
                .collect(),
        }
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
    use super::MatchAlgorithm;

    #[test]
    fn match_algorithm_prefix() {
        let algorithm = MatchAlgorithm::Prefix;

        // assert!(algorithm.matches_str("example text", ""));
        // assert!(algorithm.matches_str("example text", "examp"));
        // assert!(!algorithm.matches_str("example text", "text"));

        // assert_eq!(
        //     vec![0],
        //     algorithm.filter_u8(vec![(&[1, 2, 3], 0)], &[], false)
        // );

        // assert!(algorithm.matches_u8(&[1, 2, 3], &[]));
        // assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 2]));
        // assert!(!algorithm.matches_u8(&[1, 2, 3], &[2, 3]));
    }

    #[test]
    fn match_algorithm_fuzzy() {
        let algorithm = MatchAlgorithm::Fuzzy;

        // assert!(algorithm.matches_str("example text", ""));
        // assert!(algorithm.matches_str("example text", "examp"));
        // assert!(algorithm.matches_str("example text", "ext"));
        // assert!(algorithm.matches_str("example text", "mplxt"));
        // assert!(!algorithm.matches_str("example text", "mpp"));

        // assert!(algorithm.matches_u8(&[1, 2, 3], &[]));
        // assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 2]));
        // assert!(algorithm.matches_u8(&[1, 2, 3], &[2, 3]));
        // assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 3]));
        // assert!(!algorithm.matches_u8(&[1, 2, 3], &[2, 2]));
    }
}
