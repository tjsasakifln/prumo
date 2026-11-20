//! Finite binary64 fields across Serde's buffered, tagged JSON commands.
use serde::{Deserialize, Deserializer};

pub(crate) fn scalar<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    let value = number
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            <D::Error as serde::de::Error>::custom(
                "A grandeza deve ser finita e representável em f64.",
            )
        })?;
    if value == 0.0
        && number
            .to_string()
            .split(['e', 'E'])
            .next()
            .unwrap_or("")
            .bytes()
            .any(|byte| (b'1'..=b'9').contains(&byte))
    {
        return Err(serde::de::Error::custom(
            "A conversão numérica apagaria um valor não nulo por underflow.",
        ));
    }
    Ok(value)
}

#[derive(Deserialize)]
#[serde(transparent)]
struct Finite(#[serde(deserialize_with = "scalar")] f64);

pub(crate) fn vector3<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[f64; 3], D::Error> {
    Ok(<[Finite; 3]>::deserialize(deserializer)?.map(|value| value.0))
}

pub(crate) fn matrix3<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<[[f64; 3]; 3], D::Error> {
    Ok(<[[Finite; 3]; 3]>::deserialize(deserializer)?.map(|row| row.map(|value| value.0)))
}
