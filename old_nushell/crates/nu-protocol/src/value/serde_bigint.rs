use num_bigint::BigInt;

/// Enable big int serialization by providing a `serialize` function
pub fn serialize<S>(big_int: &BigInt, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde::Serialize::serialize(&big_int.to_string(), serializer)
}

/// Enable big int deserialization by providing a `deserialize` function
pub fn deserialize<'de, D>(deserializer: D) -> Result<BigInt, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x: String = serde::Deserialize::deserialize(deserializer)?;

    BigInt::parse_bytes(x.as_bytes(), 10)
        .ok_or_else(|| serde::de::Error::custom("expected a bignum"))
}
