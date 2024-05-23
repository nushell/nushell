use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nu_parser::trim_quotes_str;
use nu_protocol::CompletionAlgorithm;
use nu_utils::IgnoreCaseExt;
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
    /// Keeps only items that match the given `needle`
    ///
    /// # Arguments
    ///
    /// * `items` - A list of haystacks and their corresponding items
    /// * `needle` - The text to search for
    /// * `case_sensitive` - true to respect case, false to ignore it
    ///
    /// # Returns
    ///
    /// A list of matching items, as well as the indices in their haystacks that matched
    pub fn filter_str_with_inds<T>(
        &self,
        items: Vec<(impl AsRef<str>, T)>,
        needle: &str,
        case_sensitive: bool,
    ) -> Vec<(T, Vec<usize>)> {
        let needle = trim_quotes_str(needle);
        match *self {
            MatchAlgorithm::Prefix => {
                let needle = if case_sensitive {
                    Cow::Borrowed(needle)
                } else {
                    Cow::Owned(needle.to_folded_case())
                };
                items
                    .into_iter()
                    .filter_map(|(haystack, item)| {
                        if trim_quotes_str(haystack.as_ref()).starts_with(needle.as_ref()) {
                            Some((item, (0..needle.len()).collect()))
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            MatchAlgorithm::Fuzzy => filter_fuzzy(items, needle, case_sensitive),
        }
    }

    pub fn filter_str<T>(
        &self,
        items: Vec<(impl AsRef<str>, T)>,
        needle: &str,
        case_sensitive: bool,
    ) -> Vec<T> {
        let (matches, _): (Vec<_>, Vec<_>) = self
            .filter_str_with_inds(items, needle, case_sensitive)
            .into_iter()
            .unzip();
        matches
    }

    pub fn filter_u8<T>(
        &self,
        items: Vec<(impl AsRef<[u8]>, T)>,
        needle: &[u8],
        case_sensitive: bool,
    ) -> Vec<T> {
        match *self {
            MatchAlgorithm::Prefix => {
                let needle = if case_sensitive {
                    Cow::Borrowed(needle)
                } else {
                    Cow::Owned(needle.to_ascii_lowercase())
                };
                items
                    .into_iter()
                    .filter_map(|(haystack, item)| {
                        if haystack.as_ref().starts_with(&needle) {
                            Some(item)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            MatchAlgorithm::Fuzzy => {
                let items = items
                    .into_iter()
                    .map(|(haystack, item)| {
                        (String::from_utf8_lossy(haystack.as_ref()).to_string(), item)
                    })
                    .collect();
                let (matches, _): (Vec<_>, Vec<_>) =
                    filter_fuzzy(items, &String::from_utf8_lossy(needle), case_sensitive)
                        .into_iter()
                        .unzip();
                matches
            }
        }
    }
}

pub fn filter_fuzzy<T>(
    items: Vec<(impl AsRef<str>, T)>,
    needle: &str,
    case_sensitive: bool,
) -> Vec<(T, Vec<usize>)> {
    let matcher = if case_sensitive {
        SkimMatcherV2::default().respect_case()
    } else {
        SkimMatcherV2::default().ignore_case()
    };
    items
        .into_iter()
        .filter_map(|(haystack, item)| {
            matcher
                .fuzzy_indices(trim_quotes_str(haystack.as_ref()), needle)
                .map(|(_score, inds)| (item, inds))
        })
        .collect()
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

        assert_eq!(
            vec![0],
            algorithm.filter_u8(vec![(&[1, 2, 3], 0)], &[], false)
        );

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
