use std::cmp::Ordering;
use unicase::UniCase;

pub trait IgnoreCaseExt {
    /// Returns a [case folded] equivalent of this string, as a new String.
    ///
    /// Case folding is primarily based on lowercase mapping, but includes
    /// additional changes to the source text to help make case folding
    /// language-invariant and consistent. Case folded text should be used
    /// solely for processing and generally should not be stored or displayed.
    ///
    /// Note: this method might only do [`str::to_lowercase`] instead of a
    /// full case fold, depending on how Nu is compiled. You should still
    /// prefer using this method for generating case-insensitive strings,
    /// though, as it expresses intent much better than `to_lowercase`.
    ///
    /// [case folded]: <https://unicode.org/faq/casemap_charprop.html#2>
    fn to_folded_case(&self) -> String;

    /// Checks that two strings are a case-insensitive match.
    ///
    /// Essentially `to_folded_case(a) == to_folded_case(b)`, but without
    /// allocating and copying string temporaries. Because case folding involves
    /// Unicode table lookups, it can sometimes be more efficient to use
    /// `to_folded_case` to case fold once and then compare those strings.
    fn eq_ignore_case(&self, other: &str) -> bool;

    /// Compares two strings case-insensitively.
    ///
    /// Essentially `to_folded_case(a) == to_folded_case(b)`, but without
    /// allocating and copying string temporaries. Because case folding involves
    /// Unicode table lookups, it can sometimes be more efficient to use
    /// `to_folded_case` to case fold once and then compare those strings.
    ///
    /// Note that this *only* ignores case, comparing the folded strings without
    /// any other collation data or locale, so the sort order may be surprising
    /// outside of ASCII characters.
    fn cmp_ignore_case(&self, other: &str) -> Ordering;
}

impl IgnoreCaseExt for str {
    fn to_folded_case(&self) -> String {
        // we only do to_lowercase, as unicase doesn't expose its case fold yet
        // (seanmonstar/unicase#61) and we don't want to pull in another table
        self.to_lowercase()
    }

    fn eq_ignore_case(&self, other: &str) -> bool {
        UniCase::new(self) == UniCase::new(other)
    }

    fn cmp_ignore_case(&self, other: &str) -> Ordering {
        UniCase::new(self).cmp(&UniCase::new(other))
    }
}
