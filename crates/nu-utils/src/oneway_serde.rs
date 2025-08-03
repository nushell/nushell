use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum OnewaySerde<B, O> {
    Borrowed(B),
    Owned(O),
}

impl<'de, B, O> Deserialize<'de> for OnewaySerde<B, O>
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
        type Strings = OnewaySerde<&'static [&'static str], Vec<String>>;

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
