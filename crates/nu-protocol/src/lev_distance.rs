// This file is copied from the rust compiler project:
// https://github.com/rust-lang/rust/blob/cf9ed0dd5836201843d28bbad50abfbe1913af2a/compiler/rustc_span/src/lev_distance.rs#L1
// https://github.com/rust-lang/rust/blob/cf9ed0dd5836201843d28bbad50abfbe1913af2a/LICENSE-MIT
//
// - the rust compiler-specific symbol::Symbol has been replaced by &str
// - unstable feature .then_some has been replaced by an if ... else expression

//! Levenshtein distances.
//!
//! The [Levenshtein distance] is a metric for measuring the difference between two strings.
//!
//! [Levenshtein distance]: https://en.wikipedia.org/wiki/Levenshtein_distance

use std::cmp;

/// Finds the Levenshtein distance between two strings.
///
/// Returns None if the distance exceeds the limit.
pub fn lev_distance(a: &str, b: &str, limit: usize) -> Option<usize> {
    let n = a.chars().count();
    let m = b.chars().count();
    let min_dist = if n < m { m - n } else { n - m };

    if min_dist > limit {
        return None;
    }
    if n == 0 || m == 0 {
        return Some(min_dist);
    }

    let mut dcol: Vec<_> = (0..=m).collect();

    for (i, sc) in a.chars().enumerate() {
        let mut current = i;
        dcol[0] = current + 1;

        for (j, tc) in b.chars().enumerate() {
            let next = dcol[j + 1];
            if sc == tc {
                dcol[j + 1] = current;
            } else {
                dcol[j + 1] = cmp::min(current, next);
                dcol[j + 1] = cmp::min(dcol[j + 1], dcol[j]) + 1;
            }
            current = next;
        }
    }

    if dcol[m] <= limit {
        Some(dcol[m])
    } else {
        None
    }
}

/// Finds the Levenshtein distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    lev_distance(a, b, usize::max_value()).unwrap_or(usize::max_value())
}
/// Provides a word similarity score between two words that accounts for substrings being more
/// meaningful than a typical Levenshtein distance. The lower the score, the closer the match.
/// 0 is an identical match.
///
/// Uses the Levenshtein distance between the two strings and removes the cost of the length
/// difference. If this is 0 then it is either a substring match or a full word match, in the
/// substring match case we detect this and return `1`. To prevent finding meaningless substrings,
/// eg. "in" in "shrink", we only perform this subtraction of length difference if one of the words
/// is not greater than twice the length of the other. For cases where the words are close in size
/// but not an exact substring then the cost of the length difference is discounted by half.
///
/// Returns `None` if the distance exceeds the limit.
pub fn lev_distance_with_substrings(a: &str, b: &str, limit: usize) -> Option<usize> {
    let n = a.chars().count();
    let m = b.chars().count();

    // Check one isn't less than half the length of the other. If this is true then there is a
    // big difference in length.
    let big_len_diff = (n * 2) < m || (m * 2) < n;
    let len_diff = if n < m { m - n } else { n - m };
    let lev = lev_distance(a, b, limit + len_diff)?;

    // This is the crux, subtracting length difference means exact substring matches will now be 0
    let score = lev - len_diff;

    // If the score is 0 but the words have different lengths then it's a substring match not a full
    // word match
    let score = if score == 0 && len_diff > 0 && !big_len_diff {
        1 // Exact substring match, but not a total word match so return non-zero
    } else if !big_len_diff {
        // Not a big difference in length, discount cost of length difference
        score + (len_diff + 1) / 2
    } else {
        // A big difference in length, add back the difference in length to the score
        score + len_diff
    };

    if score <= limit {
        Some(score)
    } else {
        None
    }
}

/// Finds the best match for given word in the given iterator where substrings are meaningful.
///
/// A version of [`find_best_match_for_name`] that uses [`lev_distance_with_substrings`] as the score
/// for word similarity. This takes an optional distance limit which defaults to one-third of the
/// given word.
///
/// Besides the modified Levenshtein, we use case insensitive comparison to improve accuracy
/// on an edge case with a lower(upper)case letters mismatch.
pub fn find_best_match_for_name_with_substrings<'c>(
    candidates: &[&'c str],
    lookup: &str,
    dist: Option<usize>,
) -> Option<&'c str> {
    find_best_match_for_name_impl(true, candidates, lookup, dist)
}

/// Finds the best match for a given word in the given iterator.
///
/// As a loose rule to avoid the obviously incorrect suggestions, it takes
/// an optional limit for the maximum allowable edit distance, which defaults
/// to one-third of the given word.
///
/// Besides Levenshtein, we use case insensitive comparison to improve accuracy
/// on an edge case with a lower(upper)case letters mismatch.
#[allow(dead_code)]
pub fn find_best_match_for_name<'c>(
    candidates: &[&'c str],
    lookup: &str,
    dist: Option<usize>,
) -> Option<&'c str> {
    find_best_match_for_name_impl(false, candidates, lookup, dist)
}

#[cold]
fn find_best_match_for_name_impl<'c>(
    use_substring_score: bool,
    candidates: &[&'c str],
    lookup: &str,
    dist: Option<usize>,
) -> Option<&'c str> {
    let lookup_uppercase = lookup.to_uppercase();

    // Priority of matches:
    // 1. Exact case insensitive match
    // 2. Levenshtein distance match
    // 3. Sorted word match
    if let Some(c) = candidates
        .iter()
        .find(|c| c.to_uppercase() == lookup_uppercase)
    {
        return Some(*c);
    }

    let mut dist = dist.unwrap_or_else(|| cmp::max(lookup.len(), 3) / 3);
    let mut best = None;
    for c in candidates {
        let lev_dist = if use_substring_score {
            lev_distance_with_substrings(lookup, c, dist)
        } else {
            lev_distance(lookup, c, dist)
        };
        match lev_dist {
            Some(0) => return Some(*c),
            Some(d) => {
                dist = d - 1;
                best = Some(*c);
            }
            None => {}
        }
    }
    if best.is_some() {
        return best;
    }

    find_match_by_sorted_words(candidates, lookup)
}

fn find_match_by_sorted_words<'c>(iter_names: &[&'c str], lookup: &str) -> Option<&'c str> {
    iter_names.iter().fold(None, |result, candidate| {
        if sort_by_words(candidate) == sort_by_words(lookup) {
            Some(*candidate)
        } else {
            result
        }
    })
}

fn sort_by_words(name: &str) -> String {
    let mut split_words: Vec<&str> = name.split('_').collect();
    // We are sorting primitive &strs and can use unstable sort here.
    split_words.sort_unstable();
    split_words.join("_")
}

// This file is copied from the rust compiler project:
// https://github.com/rust-lang/rust/blob/cf9ed0dd5836201843d28bbad50abfbe1913af2a/compiler/rustc_span/src/lev_distance.rs#L1
// https://github.com/rust-lang/rust/blob/cf9ed0dd5836201843d28bbad50abfbe1913af2a/LICENSE-MIT

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
// Footer
