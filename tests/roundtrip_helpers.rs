//! Test-only helpers shared by every round-trip suite.
//!
//! Included by each `tests/*.rs` crate via:
//! `#[path = "roundtrip_helpers.rs"] mod helpers;`
//!
//! This file is also compiled standalone as an (empty) integration-test
//! target, hence the `dead_code` allowance.

#![allow(dead_code)]

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Serializes a builder to its JSON value form.
pub fn to_value<B: Serialize>(builder: &B) -> Value {
    serde_json::to_value(builder).expect("builder serializes")
}

/// Deserializes a builder from its JSON value form through wrapper `W`.
pub fn rebuild<W, B>(json: &Value) -> Value
where
    W: DeserializeOwned + Into<B>,
    B: Serialize,
{
    let wrapper: W =
        serde_json::from_value(json.clone()).expect("wrapper deserializes from JSON value");
    let builder: B = wrapper.into();
    serde_json::to_value(&builder).expect("rebuilt builder serializes")
}

/// Round-trips a builder through wrapper `W` (deserialize → convert) and
/// asserts the rebuilt builder serializes to the exact same JSON.
///
/// Panics with pretty-printed both sides on mismatch.
pub fn assert_roundtrip<W, B>(builder: &B)
where
    W: DeserializeOwned + Into<B>,
    B: Serialize + Debug,
{
    let original = to_value(builder);
    let rebuilt = rebuild::<W, B>(&original);
    assert_value_eq(&original, &rebuilt);
}

/// Asserts two JSON values are equal, panicking with a readable diff.
pub fn assert_value_eq(expected: &Value, actual: &Value) {
    if expected != actual {
        panic!(
            "round-trip mismatch\nexpected:\n{}\nactual:\n{}",
            pretty(expected),
            pretty(actual)
        );
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).expect("value prints")
}

/// Same as [`rebuild`] for a JSON array of components.
pub fn rebuild_vec<W, B>(json: &Value) -> Value
where
    W: DeserializeOwned + Into<B>,
    B: Serialize,
{
    let wrappers: Vec<W> =
        serde_json::from_value(json.clone()).expect("wrappers deserialize from JSON value");
    let builders: Vec<B> = wrappers.into_iter().map(Into::into).collect();
    serde_json::to_value(&builders).expect("rebuilt builders serialize")
}
