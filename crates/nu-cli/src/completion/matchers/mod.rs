pub(crate) mod case_sensitive;
pub(crate) mod naive_case_insensitive;
pub(crate) mod unicode_case_insensitive;

pub trait Matcher {
    fn matches (
        &self,
        partial: &str,
        from: &str
    ) -> bool;
}