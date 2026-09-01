//! JSON parity tests for `{ expr }` splices in all D5 parent positions.
//!
//! Each test exercises the check-then-build expansion path by comparing the
//! macro output against a hand-built oracle (serde_json comparison).  The
//! literal-only parents are covered by `view_macros.rs`; this file covers
//! the spliced code path only.

use pwr_ext::view;
use pwr_ext::view_support::CreateActionRow;
use pwr_ext::view_support::CreateButton;
use pwr_ext::view_support::CreateComponent;
use pwr_ext::view_support::CreateContainer;
use pwr_ext::view_support::CreateContainerComponent;
use pwr_ext::view_support::CreateEmbed;
use pwr_ext::view_support::CreateMediaGallery;
use pwr_ext::view_support::CreateMediaGalleryItem;
use pwr_ext::view_support::CreateMessage;
use pwr_ext::view_support::CreateSection;
use pwr_ext::view_support::CreateSectionAccessory;
use pwr_ext::view_support::CreateSectionComponent;
use pwr_ext::view_support::CreateTextDisplay;
use pwr_ext::view_support::CreateThumbnail;
use pwr_ext::view_support::CreateUnfurledMediaItem;
use pwr_ext::view_support::MessageFlags;
use serde_json::Value;

fn to_value(msg: CreateMessage<'static>) -> Value {
    serde_json::to_value(&msg).unwrap()
}

// ── helpers ────────────────────────────────────────────────────────────────

fn nav_buttons(count: usize) -> Vec<CreateButton<'static>> {
    (0..count)
        .map(|i| CreateButton::new(format!("nav:{i}")).label(format!("Nav {i}")))
        .collect()
}

fn section_texts(count: usize) -> Vec<CreateSectionComponent<'static>> {
    (0..count)
        .map(|i| CreateSectionComponent::TextDisplay(CreateTextDisplay::new(format!("line {i}"))))
        .collect()
}

fn container_texts(count: usize) -> Vec<CreateContainerComponent<'static>> {
    (0..count)
        .map(|i| CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!("cell {i}"))))
        .collect()
}

fn gallery_items(count: usize) -> Vec<CreateMediaGalleryItem<'static>> {
    (0..count)
        .map(|i| {
            CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(format!(
                "attachment://img-{i}.png"
            )))
        })
        .collect()
}

fn root_texts(count: usize) -> Vec<CreateComponent<'static>> {
    (0..count)
        .map(|i| CreateComponent::TextDisplay(CreateTextDisplay::new(format!("root {i}"))))
        .collect()
}

fn legacy_rows(count: usize) -> Vec<CreateComponent<'static>> {
    (0..count)
        .map(|i| {
            CreateComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new(format!("legacy:{i}")).label(format!("Row {i}")),
            ]))
        })
        .collect()
}

fn embed_fields(count: usize) -> Vec<(String, String, bool)> {
    (0..count)
        .map(|i| (format!("Field {i}"), format!("value {i}"), i % 2 == 0))
        .collect()
}

// ── container – positional splice ──────────────────────────────────────────

#[test]
fn container_positional_splice_parity() {
    let extra = container_texts(2);
    let via_macro = view! {
        components_v2 {
            container {
                text_display { "first" }
                { extra }
                text_display { "last" }
            }
        }
    };
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::Container(CreateContainer::new(
            vec![
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("first")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("cell 0")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("cell 1")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("last")),
            ],
        ))]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── container – Option<T> splice (conditional) ─────────────────────────────

#[test]
fn container_option_some_splice_parity() {
    let maybe: Option<CreateContainerComponent<'static>> = Some(
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new("opt")),
    );
    let via_macro = view! {
        components_v2 {
            container {
                text_display { "a" }
                { maybe }
                text_display { "b" }
            }
        }
    };
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::Container(CreateContainer::new(
            vec![
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("a")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("opt")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("b")),
            ],
        ))]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

#[test]
fn container_option_none_splice_parity() {
    let maybe: Option<CreateContainerComponent<'static>> = None;
    let via_macro = view! {
        components_v2 {
            container {
                text_display { "a" }
                { maybe }
                text_display { "b" }
            }
        }
    };
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::Container(CreateContainer::new(
            vec![
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("a")),
                CreateContainerComponent::TextDisplay(CreateTextDisplay::new("b")),
            ],
        ))]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── action_row – positional splice ─────────────────────────────────────────

#[test]
fn action_row_positional_splice_parity() {
    let extra = nav_buttons(2);
    let via_macro = view! {
        action_row {
            button { custom_id: "b1", label: "B1" }
            { extra }
            button { custom_id: "b4", label: "B4" }
        }
    };
    let expected = CreateMessage::new().components(vec![CreateComponent::ActionRow(
        CreateActionRow::buttons(vec![
            CreateButton::new("b1").label("B1"),
            CreateButton::new("nav:0").label("Nav 0"),
            CreateButton::new("nav:1").label("Nav 1"),
            CreateButton::new("b4").label("B4"),
        ]),
    )]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── v2 root – positional splice ────────────────────────────────────────────

#[test]
fn v2_root_positional_splice_parity() {
    let extra = root_texts(1);
    let via_macro = view! {
        components_v2 {
            text_display { "a" }
            { extra }
            text_display { "b" }
        }
    };
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![
            CreateComponent::TextDisplay(CreateTextDisplay::new("a")),
            CreateComponent::TextDisplay(CreateTextDisplay::new("root 0")),
            CreateComponent::TextDisplay(CreateTextDisplay::new("b")),
        ]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── legacy root – positional splice (components) ───────────────────────────

#[test]
fn legacy_root_positional_splice_parity() {
    let extra = legacy_rows(1);
    let via_macro = view! {
        content: "hello"
        action_row {
            button { custom_id: "b1", label: "B1" }
        }
        { extra }
    };
    let expected = CreateMessage::new().content("hello").components(vec![
        CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            CreateButton::new("b1").label("B1"),
        ])),
        CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            CreateButton::new("legacy:0").label("Row 0"),
        ])),
    ]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── section – positional splice ────────────────────────────────────────────

#[test]
fn section_positional_splice_parity() {
    let extra = section_texts(1);
    let via_macro = view! {
        components_v2 {
            section {
                text_display { "first" }
                { extra }
                text_display { "last" }
                thumbnail { media: "https://example.test/t.png" }
            }
        }
    };
    let texts = vec![
        CreateSectionComponent::TextDisplay(CreateTextDisplay::new("first")),
        CreateSectionComponent::TextDisplay(CreateTextDisplay::new("line 0")),
        CreateSectionComponent::TextDisplay(CreateTextDisplay::new("last")),
    ];
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::Section(CreateSection::new(
            texts,
            CreateSectionAccessory::Thumbnail(CreateThumbnail::new(CreateUnfurledMediaItem::new(
                "https://example.test/t.png",
            ))),
        ))]);
    assert_eq!(to_value(via_macro), to_value(expected));
}
#[test]
fn media_gallery_positional_splice_parity() {
    let extra = gallery_items(2);
    let via_macro = view! {
        components_v2 {
            media_gallery {
                media_gallery_item { media: "attachment://first.png" }
                { extra }
                media_gallery_item { media: "attachment://last.png" }
            }
        }
    };
    let items = vec![
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://first.png")),
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://img-0.png")),
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://img-1.png")),
        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new("attachment://last.png")),
    ];
    let expected = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::MediaGallery(
            CreateMediaGallery::new(items),
        )]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── embed fields – positional splice ───────────────────────────────────────

#[test]
fn embed_fields_positional_splice_parity() {
    let extra = embed_fields(2);
    let via_macro = view! {
        embed {
            title: "T",
            field { name: "A", value: "a", inline: true }
            { extra }
            field { name: "Z", value: "z", inline: false }
        }
    };
    let expected = CreateMessage::new().embeds(vec![
        CreateEmbed::new()
            .title("T")
            .field("A", "a", true)
            .field("Field 0", "value 0", true)
            .field("Field 1", "value 1", false)
            .field("Z", "z", false),
    ]);
    assert_eq!(to_value(via_macro), to_value(expected));
}

// ── runtime law checks fire for spliced parents ────────────────────────────

#[test]
#[should_panic(expected = "action_row cannot contain more than 5 buttons")]
fn action_row_splice_over_limit_panics_at_runtime() {
    let _ = view! {
        action_row {
            { nav_buttons(6) }
        }
    };
}

#[test]
#[should_panic(expected = "section cannot contain more than 3 text_display components")]
fn section_splice_over_limit_panics_at_runtime() {
    let _ = view! {
        components_v2 {
            section {
                { section_texts(4) }
                thumbnail { media: "https://example.test/t.png" }
            }
        }
    };
}

#[test]
#[should_panic(expected = "media_gallery must contain at least one `media_gallery_item`")]
fn media_gallery_splice_empty_panics_at_runtime() {
    let _ = view! {
        components_v2 {
            media_gallery {
                { Vec::<CreateMediaGalleryItem<'static>>::new() }
            }
        }
    };
}

#[test]
#[should_panic(expected = "components_v2 cannot contain more than 40 components")]
fn v2_root_splice_over_limit_panics_at_runtime() {
    let _ = view! {
        components_v2 {
            { root_texts(41) }
        }
    };
}

#[test]
#[should_panic(expected = "`components_v2` must contain at least one component")]
fn v2_root_splice_empty_panics_at_runtime() {
    let _ = view! {
        components_v2 {
            { Vec::<CreateComponent<'static>>::new() }
        }
    };
}

#[test]
#[should_panic(expected = "container cannot contain more than 40 components")]
fn container_splice_over_limit_panics_at_runtime() {
    let _ = view! {
        components_v2 {
            container {
                { container_texts(41) }
            }
        }
    };
}

// ── order-independent count guard: splice anywhere defers to runtime ───────

#[test]
#[should_panic(expected = "action_row cannot contain more than 5 buttons")]
fn action_row_literal_over_limit_with_leading_splice_panics_at_runtime() {
    // 6 literal buttons + a leading splice: the compile-time guard must
    // NOT fire (the splice exists), so the row compiles and panics at
    // runtime when the check sees 6 buttons.
    let extra = nav_buttons(0);
    let _ = view! {
        action_row {
            { extra }
            button { custom_id: "b1", label: "B1" }
            button { custom_id: "b2", label: "B2" }
            button { custom_id: "b3", label: "B3" }
            button { custom_id: "b4", label: "B4" }
            button { custom_id: "b5", label: "B5" }
            button { custom_id: "b6", label: "B6" }
        }
    };
}

#[test]
#[should_panic(expected = "action_row cannot contain more than 5 buttons")]
fn action_row_literal_over_limit_with_trailing_splice_panics_at_runtime() {
    // 6 literal buttons + a trailing splice: same outcome — the guard
    // scans ALL items, so the trailing splice is seen and the row is
    // routed to the runtime check, producing the same panic.
    let extra = nav_buttons(0);
    let _ = view! {
        action_row {
            button { custom_id: "b1", label: "B1" }
            button { custom_id: "b2", label: "B2" }
            button { custom_id: "b3", label: "B3" }
            button { custom_id: "b4", label: "B4" }
            button { custom_id: "b5", label: "B5" }
            button { custom_id: "b6", label: "B6" }
            { extra }
        }
    };
}

#[test]
#[should_panic(expected = "section cannot contain more than 3 text_display components")]
fn section_literal_over_limit_with_leading_splice_panics_at_runtime() {
    let extra = section_texts(0);
    let _ = view! {
        components_v2 {
            section {
                { extra }
                text_display { "1" }
                text_display { "2" }
                text_display { "3" }
                text_display { "4" }
                thumbnail { media: "https://example.test/t.png" }
            }
        }
    };
}

#[test]
#[should_panic(expected = "section cannot contain more than 3 text_display components")]
fn section_literal_over_limit_with_trailing_splice_panics_at_runtime() {
    let extra = section_texts(0);
    let _ = view! {
        components_v2 {
            section {
                text_display { "1" }
                text_display { "2" }
                text_display { "3" }
                text_display { "4" }
                { extra }
                thumbnail { media: "https://example.test/t.png" }
            }
        }
    };
}
