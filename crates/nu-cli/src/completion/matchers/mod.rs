pub(crate) mod case_insensitive;
pub(crate) mod case_sensitive;

pub trait Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool;
}
