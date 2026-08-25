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
use serenity::builder::DataUri;

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

/// Declares the boilerplate behind an opaque wrapper: a custom
/// `Deserialize` impl that captures the node and hands it to the wrapper's
/// `parse_*` function, plus the identity `From<Wrapper> for Builder`
/// conversion over the tuple field.
macro_rules! opaque_wrapper {
    ($wrapper:ident, $inner:ty, $parse:expr) => {
        impl From<$wrapper> for $inner {
            fn from(de: $wrapper) -> Self {
                de.0
            }
        }

        impl<'de> serde::Deserialize<'de> for $wrapper {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = crate::util::capture_value(deserializer)?;
                $parse(&value)
                    .map_err(<D::Error as serde::de::Error>::custom)
                    .map($wrapper)
            }
        }
    };
}

pub(crate) use opaque_wrapper;

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

impl DataUriDe {
    /// Converts into upstream's [`DataUri`]. Total: deserialization already
    /// enforced the exact URI grammar the upstream constructor re-checks, so
    /// this cannot fail.
    pub(crate) fn into_data_uri(self) -> DataUri<'static> {
        DataUri::from_base64(self.0).expect("data URI validated during deserialization")
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
