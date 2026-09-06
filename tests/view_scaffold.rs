use pwr_ext::view;
use serde_json::json;

#[test]
fn view_empty_is_empty_message() {
    let msg = view! {};
    let value = serde_json::to_value(&msg).unwrap();
    // CreateMessage::new() serializes with empty content defaults; verify it round-trips as object
    assert!(value.is_object());
}

#[test]
fn view_content_sets_content() {
    let msg = view! { content: "hi" };
    let value = serde_json::to_value(&msg).unwrap();
    assert_eq!(value["content"], json!("hi"));
}

#[test]
fn view_content_trailing_comma() {
    let msg = view! { content: "hi", };
    let value = serde_json::to_value(&msg).unwrap();
    assert_eq!(value["content"], json!("hi"));
}

#[test]
fn view_content_expression() {
    let text = String::from("hello");
    let msg = view! { content: text.clone() };
    let value = serde_json::to_value(&msg).unwrap();
    assert_eq!(value["content"], json!("hello"));
}

#[test]
fn view_literal_only_yields_bare_builder() {
    // A literal-only view must NOT be wrapped in Result — binding against
    // the bare builder type compiles only if the expansion is unwrapped.
    let msg: ::pwr_ext::view_support::CreateMessage = view! {
        content: "hi"
    };
    let value = serde_json::to_value(&msg).unwrap();
    assert_eq!(value["content"], json!("hi"));
}

#[test]
fn view_v2_literal_only_yields_bare_builder() {
    // Same pin for the components v2 family: a literal-only v2 view stays
    // the bare builder.
    let msg: ::pwr_ext::view_support::CreateMessage = view! {
        components_v2 {
            text_display { "x" }
        }
    };
    let value = serde_json::to_value(&msg).unwrap();
    assert_eq!(value["components"][0]["content"], json!("x"));
}
