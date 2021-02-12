use bigdecimal::BigDecimal;

/// Enable big decimal serialization by providing a `serialize` function
pub fn serialize<S>(big_decimal: &BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde::Serialize::serialize(&big_decimal.to_string(), serializer)
}

/// Enable big decimal deserialization by providing a `deserialize` function
pub fn deserialize<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x: String = serde::Deserialize::deserialize(deserializer)?;
    BigDecimal::parse_bytes(x.as_bytes(), 10)
        .ok_or_else(|| serde::de::Error::custom("expected a bigdecimal"))
}
