pub(crate) mod case_sensitive;
pub(crate) mod case_insensitive;

pub trait Matcher {
    fn matches(&self, partial: &str, from: &str) -> bool;
}
