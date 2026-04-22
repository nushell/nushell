use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
    hash::Hash,
    ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive},
};

/// A minimal abstraction for membership checks across container-like types.
///
/// This trait unifies `contains` behavior for common collections, strings,
/// ranges, and maps, letting generic code ask "does this container contain
/// this item?" without caring about the concrete type.
///
/// The associated `Item` type represents the element or key being queried.
/// For maps, this is the key type; for `str`/`String`, it is `str`.
pub trait Container {
    type Item: ?Sized;

    fn contains(&self, item: &Self::Item) -> bool;
}

impl<C: Container + ?Sized> Container for &C {
    type Item = C::Item;

    fn contains(&self, item: &Self::Item) -> bool {
        C::contains(self, item)
    }
}

impl Container for str {
    type Item = str;

    fn contains(&self, item: &str) -> bool {
        str::contains(self, item)
    }
}

impl Container for String {
    type Item = str;

    fn contains(&self, item: &Self::Item) -> bool {
        self.as_str().contains(item)
    }
}

impl<T: PartialEq, const N: usize> Container for [T; N] {
    type Item = T;

    fn contains(&self, item: &Self::Item) -> bool {
        self.as_slice().contains(item)
    }
}

impl<T: PartialEq> Container for [T] {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        <[T]>::contains(self, item)
    }
}

impl<T: PartialEq> Container for Vec<T> {
    type Item = T;

    fn contains(&self, item: &Self::Item) -> bool {
        self.as_slice().contains(item)
    }
}

impl<T: PartialEq> Container for VecDeque<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        VecDeque::contains(self, item)
    }
}

impl<T: PartialEq> Container for LinkedList<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        LinkedList::contains(self, item)
    }
}

impl<T: Eq + Hash> Container for HashSet<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        HashSet::contains(self, item)
    }
}

impl<T: Ord> Container for BTreeSet<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        BTreeSet::contains(self, item)
    }
}

impl<K: Eq + Hash, V> Container for HashMap<K, V> {
    type Item = K;

    fn contains(&self, item: &K) -> bool {
        HashMap::contains_key(self, item)
    }
}

impl<K: Ord, V> Container for BTreeMap<K, V> {
    type Item = K;

    fn contains(&self, item: &K) -> bool {
        BTreeMap::contains_key(self, item)
    }
}

impl<T: PartialOrd> Container for Range<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        Range::contains(self, item)
    }
}

impl<T: PartialOrd> Container for RangeInclusive<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        RangeInclusive::contains(self, item)
    }
}

impl<T: PartialOrd> Container for RangeFrom<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        RangeFrom::contains(self, item)
    }
}

impl<T: PartialOrd> Container for RangeTo<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        RangeTo::contains(self, item)
    }
}

impl<T: PartialOrd> Container for RangeToInclusive<T> {
    type Item = T;

    fn contains(&self, item: &T) -> bool {
        RangeToInclusive::contains(self, item)
    }
}

impl<'a, C> Container for Cow<'a, C>
where
    C: Container + ToOwned + ?Sized,
{
    type Item = C::Item;

    fn contains(&self, item: &Self::Item) -> bool {
        self.as_ref().contains(item)
    }
}
