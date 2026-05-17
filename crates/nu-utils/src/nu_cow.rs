use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum NuCow<B, O> {
    Borrowed(B),
    Owned(O),
}

impl<B, O> PartialEq for NuCow<B, O>
where
    O: std::cmp::PartialEq<O>,
    B: std::cmp::PartialEq<B>,
    O: std::cmp::PartialEq<B>,
{
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (NuCow::Owned(o), NuCow::Borrowed(b)) | (NuCow::Borrowed(b), NuCow::Owned(o)) => o == b,
            (NuCow::Borrowed(lhs), NuCow::Borrowed(rhs)) => lhs == rhs,
            (NuCow::Owned(lhs), NuCow::Owned(rhs)) => lhs == rhs,
        }
    }
}

impl<B, O> Debug for NuCow<B, O>
where
    B: Debug,
    O: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self {
                Self::Borrowed(b) => f.debug_tuple("Borrowed").field(b).finish(),
                Self::Owned(o) => f.debug_tuple("Owned").field(o).finish(),
            }
        } else {
            match self {
                Self::Borrowed(b) => <B as Debug>::fmt(b, f),
                Self::Owned(o) => <O as Debug>::fmt(o, f),
            }
        }
    }
}

impl<B, O> Clone for NuCow<B, O>
where
    B: Clone,
    O: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(b) => Self::Borrowed(b.clone()),
            Self::Owned(o) => Self::Owned(o.clone()),
        }
    }
}

impl<'de, B, O> Deserialize<'de> for NuCow<B, O>
where
    O: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <O as Deserialize>::deserialize(deserializer).map(Self::Owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_to_dynamic_roundtrip() {
        type Strings = NuCow<&'static [&'static str], Vec<String>>;

        let src = ["hello", "world", "!"].as_slice();
        let json = serde_json::to_string(&Strings::Borrowed(src)).unwrap();
        let dst: Strings = serde_json::from_str(&json).unwrap();

        let Strings::Owned(dst) = dst else {
            panic!("Expected Owned variant");
        };

        for (&s, d) in src.iter().zip(&dst) {
            assert_eq!(s, d.as_str())
        }
    }
}
