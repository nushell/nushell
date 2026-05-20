use nu_utils::container::Container;
use std::{borrow::Borrow, fmt::Debug};

/// Assert that a haystack contains the given needle.
///
/// Uses the [`Container`] abstraction so it works with slices, vectors, sets,
/// maps (by key), strings, and ranges.
/// The error message includes both the container and the item for quick debugging.
///
/// # Panics
///
/// Panics if `haystack.contains(needle)` returns false.
#[track_caller]
pub fn assert_contains<H, N>(needle: N, haystack: H)
where
    H: Container + Debug,
    N: Borrow<H::Item>,
    H::Item: Debug,
{
    let item = needle.borrow();

    assert!(
        haystack.contains(item),
        "{haystack:?} does not contain {item:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(clippy::needless_borrows_for_generic_args)]
    fn test_something() {
        assert_contains(1, [1, 2, 3]);
        assert_contains(2, &[1, 2, 3]);
        assert_contains("a", "abc");
        assert_contains("b", String::from("abc"));
        assert_contains(String::from("b"), String::from("abc"));
        assert_contains("c", &String::from("abc"));
        assert_contains(2, vec![1, 2, 3]);
        assert_contains(1, &vec![1, 2, 3]);
    }
}
