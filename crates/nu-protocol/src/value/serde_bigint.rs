use num_bigint::BigInt;
use num_traits::cast::FromPrimitive;
use num_traits::cast::ToPrimitive;

pub fn serialize<S>(big_int: &BigInt, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde::Serialize::serialize(
        &big_int
            .to_i64()
            .ok_or_else(|| serde::ser::Error::custom("expected a i64-sized bignum"))?,
        serializer,
    )
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<BigInt, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x: i64 = serde::Deserialize::deserialize(deserializer)?;
    Ok(BigInt::from_i64(x)
        .ok_or_else(|| serde::de::Error::custom("expected a i64-sized bignum"))?)
}
