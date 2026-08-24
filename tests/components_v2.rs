//! Round-trip suite for message components v2: sections, text displays,
//! media galleries, files, separators, containers, and the tag-dispatched
//! `CreateComponent` tree.

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateButton;
use serenity::builder::CreateComponent;
use serenity::builder::CreateContainer;
use serenity::builder::CreateContainerComponent;
use serenity::builder::CreateFile;
use serenity::builder::CreateMediaGallery;
use serenity::builder::CreateMediaGalleryItem;
use serenity::builder::CreateSection;
use serenity::builder::CreateSectionAccessory;
use serenity::builder::CreateSectionComponent;
use serenity::builder::CreateSeparator;
use serenity::builder::CreateTextDisplay;
use serenity::builder::CreateThumbnail;
use serenity::builder::CreateUnfurledMediaItem;
use serenity::model::application::ButtonStyle;
use serenity::model::application::SeparatorSpacingSize;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use helpers::rebuild_vec;
use pwr_ext::prelude::*;

// ---- Single-variant trees ----

#[test]
fn text_display_component_roundtrips() {
    let component = CreateComponent::TextDisplay(CreateTextDisplay::new(
        "**Incident #4021** — API latency elevated",
    ));

    assert_roundtrip::<CreateComponentDe, _>(&component);
}

#[test]
fn media_gallery_roundtrips_with_one_and_many_items() {
    let single =
        CreateComponent::MediaGallery(CreateMediaGallery::new(vec![CreateMediaGalleryItem::new(
            CreateUnfurledMediaItem::new("attachment://latency.png"),
        )]));

    let many = CreateComponent::MediaGallery(CreateMediaGallery::new(vec![
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://latency.png"))
            .description("5xx rate by region")
            .spoiler(false),
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
            "https://cdn.example.com/logo.png",
        ))
        .spoiler(true),
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://errors.txt")),
    ]));

    assert_roundtrip::<CreateComponentDe, _>(&single);
    assert_roundtrip::<CreateComponentDe, _>(&many);
}

#[test]
fn file_component_roundtrips() {
    let plain = CreateComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
        "attachment://report-2026-08.pdf",
    )));

    let spoiled = CreateComponent::File(
        CreateFile::new(CreateUnfurledMediaItem::new("attachment://spoilers.zip")).spoiler(true),
    );

    assert_roundtrip::<CreateComponentDe, _>(&plain);
    assert_roundtrip::<CreateComponentDe, _>(&spoiled);
}

#[test]
fn separator_with_and_without_optional_fields_roundtrips() {
    let full = CreateComponent::Separator(
        CreateSeparator::new()
            .divider(true)
            .spacing(SeparatorSpacingSize::Large),
    );
    let bare = CreateComponent::Separator(CreateSeparator::new());

    assert_roundtrip::<CreateComponentDe, _>(&full);
    assert_roundtrip::<CreateComponentDe, _>(&bare);
}

#[test]
fn every_separator_spacing_value_survives_the_roundtrip() {
    let cases = [
        json!({ "type": 14, "divider": false, "spacing": 1 }),
        json!({ "type": 14, "divider": true, "spacing": 2 }),
        // SeparatorSpacingSize carries an Unknown fallback upstream.
        json!({ "type": 14, "divider": false, "spacing": 9 }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateComponentDe, CreateComponent<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn section_with_thumbnail_accessory_roundtrips() {
    let component = CreateComponent::Section(CreateSection::new(
        vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
            "See the attached diagram.",
        ))],
        CreateSectionAccessory::Thumbnail(
            CreateThumbnail::new(CreateUnfurledMediaItem::new(
                "https://cdn.example.com/diagram.png",
            ))
            .description("Latency diagram")
            .spoiler(true),
        ),
    ));

    assert_roundtrip::<CreateComponentDe, _>(&component);
}

#[test]
fn section_with_button_accessory_roundtrips() {
    let component = CreateComponent::Section(CreateSection::new(
        vec![
            CreateSectionComponent::TextDisplay(CreateTextDisplay::new("Line one.")),
            CreateSectionComponent::TextDisplay(CreateTextDisplay::new("Line two.")),
        ],
        CreateSectionAccessory::Button(
            CreateButton::new_link("https://status.example.com").label("Open status"),
        ),
    ));

    assert_roundtrip::<CreateComponentDe, _>(&component);
}

#[test]
fn section_without_components_field_normalizes_to_empty() {
    // Upstream skips an empty components array on the wire; a payload
    // omitting it round-trips to the same shape (still no key).
    let payload = json!({
        "type": 9,
        "accessory": { "type": 11, "media": { "url": "https://cdn.example.com/x.png" } },
    });

    let rebuilt = rebuild::<CreateComponentDe, CreateComponent<'static>>(&payload);

    assert_value_eq(&payload, &rebuilt);
}

#[test]
fn action_row_inside_the_v2_tree_roundtrips() {
    let component = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new("ack:4021")
            .label("Ack")
            .style(ButtonStyle::Success)
            .disabled(true),
    ]));

    assert_roundtrip::<CreateComponentDe, _>(&component);
}

// ---- Deep nesting ----

#[test]
fn deeply_nested_container_tree_roundtrips() {
    let tree = CreateComponent::Container(
        CreateContainer::new(vec![
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "**Incident #4021** — API latency elevated in eu-central.",
            )),
            CreateContainerComponent::Section(CreateSection::new(
                vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                    "Elevated 5xx rates since 12:00 UTC.",
                ))],
                CreateSectionAccessory::Thumbnail(
                    CreateThumbnail::new(CreateUnfurledMediaItem::new(
                        "https://cdn.example.com/status-logo.png",
                    ))
                    .description("Status logo"),
                ),
            )),
            CreateContainerComponent::MediaGallery(CreateMediaGallery::new(vec![
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "attachment://latency.png",
                ))
                .description("Latency graph")
                .spoiler(false),
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "attachment://errors.png",
                )),
            ])),
            CreateContainerComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
                "attachment://postmortem.md",
            ))),
            CreateContainerComponent::Separator(
                CreateSeparator::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Small),
            ),
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new_link("https://status.example.com").label("Status page"),
                CreateButton::new("resolve:4021")
                    .label("Resolve")
                    .style(ButtonStyle::Danger),
            ])),
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "-# Updated <t:1755000000:R>",
            )),
        ])
        .accent_colour(0x58_65_F2)
        .spoiler(false),
    );

    assert_roundtrip::<CreateComponentDe, _>(&tree);
}

#[test]
fn nested_containers_are_not_representable_and_error() {
    // Discord forbids nesting containers inside containers; the builder has
    // no variant for it either.
    let payload = json!({
        "type": 17,
        "components": [{ "type": 17, "components": [] }],
    });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(
        result.is_err(),
        "a container inside a container must be rejected"
    );
}

// ---- Error paths ----

#[test]
fn unknown_top_level_type_tag_errors() {
    let payload = json!({ "type": 99, "content": "?" });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(
        result.is_err(),
        "unknown component type 99 must be rejected"
    );
}

#[test]
fn button_is_not_a_top_level_v2_component_and_errors() {
    // Buttons only exist inside action rows; there is no top-level variant.
    let payload = json!({ "type": 2, "style": 3, "custom_id": "orphan:btn" });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(result.is_err(), "a top-level button must be rejected");
}

#[test]
fn component_missing_numeric_type_tag_errors() {
    let payload = json!({ "content": "no type here" });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(result.is_err());
}

#[test]
fn text_display_missing_content_errors() {
    let payload = json!({ "type": 10 });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(
        result.is_err(),
        "a text display without content must be rejected"
    );
}

#[test]
fn section_accessory_missing_errors() {
    let payload = json!({
        "type": 9,
        "components": [{ "type": 10, "content": "orphan section" }],
    });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(
        result.is_err(),
        "a section without an accessory must be rejected"
    );
}

#[test]
fn thumbnail_missing_media_errors() {
    let payload = json!({
        "type": 9,
        "components": [],
        "accessory": { "type": 11, "description": "no media" },
    });

    let result = serde_json::from_value::<CreateComponentDe>(payload);

    assert!(result.is_err());
}

// ---- Realistic webhook-shaped payloads ----

#[test]
fn realistic_components_v2_message_array_roundtrips() {
    let payload: Value = serde_json::from_str(
        r#"[
        {
          "type": 10,
          "content": "**Incident #4021** — API latency elevated"
        },
        {
          "type": 9,
          "components": [
            { "type": 10, "content": "Elevated 5xx rates in eu-central since 12:00 UTC." }
          ],
          "accessory": {
            "type": 11,
            "media": { "url": "https://example.com/status-logo.png" },
            "description": "Status logo"
          }
        },
        {
          "type": 12,
          "items": [
            { "media": { "url": "attachment://latency.png" }, "description": "Latency graph", "spoiler": false },
            { "media": { "url": "attachment://errors.png" } }
          ]
        },
        { "type": 14, "divider": true, "spacing": 1 },
        {
          "type": 1,
          "components": [
            { "type": 2, "style": 5, "label": "Status Page", "url": "https://status.example.com", "disabled": false },
            { "type": 2, "style": 2, "custom_id": "ack_4021", "label": "Acknowledge",
              "emoji": { "animated": false, "name": "tick", "id": "936926320553408522" }, "disabled": false },
            { "type": 2, "style": 4, "custom_id": "resolve_4021", "label": "Resolve", "disabled": true }
          ]
        },
        {
          "type": 17,
          "accent_color": 15158332,
          "spoiler": false,
          "components": [
            { "type": 10, "content": "-# Updated <t:1755000000:R>" },
            { "type": 13, "file": { "url": "attachment://postmortem.md" }, "spoiler": false }
          ]
        }
      ]"#,
    )
    .expect("valid fixture JSON");

    let expected = payload.clone();
    let rebuilt = rebuild_vec::<CreateComponentDe, CreateComponent<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}
