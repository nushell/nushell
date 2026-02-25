use super::closure::Closure;
use crate::{BlockId, Value, VarId};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
};
use std::{
    borrow::{Borrow, Cow},
    fmt,
};

impl Serialize for Closure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Closure", 2)?;
        state.serialize_field("block_id", &self.block_id)?;
        state.serialize_field("captures", &self.captures)?;
        // inline_block and nested_blocks are intentionally skipped
        state.end()
    }
}

impl<'de> Deserialize<'de> for Closure {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ClosureVisitor;

        impl<'de> Visitor<'de> for ClosureVisitor {
            type Value = Closure;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Closure")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Closure, A::Error> {
                let mut block_id: Option<BlockId> = None;
                let mut captures: Option<Vec<(VarId, Value)>> = None;

                while let Some(key) = map.next_key::<Cow<str>>()? {
                    match key.borrow() {
                        "block_id" => block_id = Some(map.next_value()?),
                        "captures" => captures = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let block_id = block_id.ok_or_else(|| de::Error::missing_field("block_id"))?;
                let captures = captures.ok_or_else(|| de::Error::missing_field("captures"))?;

                Ok(Closure::new(block_id, captures))
            }
        }

        deserializer.deserialize_struct("Closure", &["block_id", "captures"], ClosureVisitor)
    }
}
