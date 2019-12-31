use bigdecimal::BigDecimal;
use num_traits::cast::FromPrimitive;
use num_traits::cast::ToPrimitive;

pub fn serialize<S>(big_decimal: &BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde::Serialize::serialize(
        &big_decimal
            .to_f64()
            .ok_or_else(|| serde::ser::Error::custom("expected a f64-sized bignum"))?,
        serializer,
    )
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x: f64 = serde::Deserialize::deserialize(deserializer)?;
    Ok(BigDecimal::from_f64(x)
        .ok_or_else(|| serde::de::Error::custom("expected a f64-sized bigdecimal"))?)
}
