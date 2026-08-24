//! Shared plumbing behind the two conversion patterns.
//!
//! - Flat builders only need [`capture_value`] (to enter their derived
//!   `Deserialize`) — or not even that.
//! - Tag-dispatched trees use [`capture_value`] + [`tag`] + [`mirror`] inside
//!   a custom [`Deserialize`](serde::Deserialize) impl.

use std::borrow::Cow;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Captures any input as an untyped [`Value`] inside a custom `Deserialize`
/// impl, for tag-dispatched tree types.
pub(crate) fn capture_value<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(deserializer)
}

/// Reads the numeric `"type"` tag every component object carries.
///
/// The tag decides the variant in tree dispatch; objects without one cannot be
/// resolved and are rejected (serenity's own `CreateActionRow` serializer
/// always writes `"type": 1`).
pub(crate) fn tag(value: &Value) -> Result<u64, String> {
    value
        .get("type")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("component missing numeric \"type\": {value}"))
}

/// Deserializes a borrowed [`Value`] into a derived mirror struct,
/// flattening serde's error into the string-based internal error type.
pub(crate) fn mirror<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    T::deserialize(value.clone()).map_err(|e| e.to_string())
}

/// Transparent mirror of upstream's [`DataUri`](serenity::builder::DataUri):
/// a plain string on the wire, validated against the same data-URI shape
/// upstream checks (`data:<type>/<subtype>;base64,<payload>`), so invalid
/// URIs error at deserialization time and conversions stay total.
#[derive(Debug)]
pub(crate) struct DataUriDe(pub Cow<'static, str>);

impl<'de> Deserialize<'de> for DataUriDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if is_valid_data_uri(&s) {
            Ok(Self(s.into()))
        } else {
            Err(serde::de::Error::custom(format!("invalid data URI: {s}")))
        }
    }
}

fn is_valid_data_uri(s: &str) -> bool {
    let Some(("data", tail)) = s.split_once(':') else {
        return false;
    };
    let Some((mimetype, encoding)) = tail.split_once(';') else {
        return false;
    };
    mimetype.split_once('/').is_some() && encoding.starts_with("base64,")
}
