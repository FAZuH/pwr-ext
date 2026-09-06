//! Law checks for runtime-assembled view children: each helper mirrors the
//! child rule the `view!` macro enforces at compile time for literal
//! children, so child lists built at runtime fail with the same law.

use std::borrow::Cow;

use pwr_ext::view_support::ChildRuleError;
use pwr_ext::view_support::CreateActionRow;
use pwr_ext::view_support::CreateButton;
use pwr_ext::view_support::CreateComponent;
use pwr_ext::view_support::CreateContainer;
use pwr_ext::view_support::CreateContainerComponent;
use pwr_ext::view_support::CreateFile;
use pwr_ext::view_support::CreateMediaGallery;
use pwr_ext::view_support::CreateMediaGalleryItem;
use pwr_ext::view_support::CreateSection;
use pwr_ext::view_support::CreateSectionAccessory;
use pwr_ext::view_support::CreateSectionComponent;
use pwr_ext::view_support::CreateSelectMenu;
use pwr_ext::view_support::CreateSelectMenuKind;
use pwr_ext::view_support::CreateSelectMenuOption;
use pwr_ext::view_support::CreateSeparator;
use pwr_ext::view_support::CreateTextDisplay;
use pwr_ext::view_support::CreateThumbnail;
use pwr_ext::view_support::CreateUnfurledMediaItem;
use pwr_ext::view_support::check_action_row_children;
use pwr_ext::view_support::check_container_children;
use pwr_ext::view_support::check_media_gallery_items;
use pwr_ext::view_support::check_section_children;
use pwr_ext::view_support::check_select_menu_options;
use pwr_ext::view_support::check_v2_root_children;
use serenity::builder::CreateLabel;

fn nav_buttons(count: usize) -> Vec<CreateButton<'static>> {
    (0..count)
        .map(|i| CreateButton::new(format!("nav:{i}")).label(format!("Nav {i}")))
        .collect()
}

fn section_button() -> CreateButton<'static> {
    CreateButton::new("hub:open".to_owned()).label("Open".to_owned())
}

fn text_display(content: &str) -> CreateTextDisplay<'static> {
    CreateTextDisplay::new(content.to_owned())
}

fn root_texts(count: usize) -> Vec<CreateComponent<'static>> {
    (0..count)
        .map(|i| CreateComponent::TextDisplay(text_display(&format!("root {i}"))))
        .collect()
}

fn section_texts(count: usize) -> Vec<CreateSectionComponent<'static>> {
    (0..count)
        .map(|i| CreateSectionComponent::TextDisplay(text_display(&format!("line {i}"))))
        .collect()
}

fn thumbnail_accessory() -> CreateSectionAccessory<'static> {
    CreateSectionAccessory::Thumbnail(CreateThumbnail::new(CreateUnfurledMediaItem::new(
        "attachment://hub.png".to_owned(),
    )))
}

fn select_options(count: usize) -> Vec<CreateSelectMenuOption<'static>> {
    (0..count)
        .map(|i| CreateSelectMenuOption::new(format!("Region {i}"), format!("r{i}")))
        .collect()
}

fn string_select_menu() -> CreateSelectMenu<'static> {
    CreateSelectMenu::new(
        "pick:region",
        CreateSelectMenuKind::String {
            options: Cow::Owned(select_options(3)),
        },
    )
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

fn container_texts(count: usize) -> Vec<CreateContainerComponent<'static>> {
    (0..count)
        .map(|i| CreateContainerComponent::TextDisplay(text_display(&format!("cell {i}"))))
        .collect()
}

fn container_all_kinds() -> Vec<CreateContainerComponent<'static>> {
    vec![
        CreateContainerComponent::TextDisplay(text_display("summary")),
        CreateContainerComponent::Section(CreateSection::new(
            section_texts(1),
            thumbnail_accessory(),
        )),
        CreateContainerComponent::ActionRow(CreateActionRow::buttons(nav_buttons(1))),
        CreateContainerComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
            "attachment://log.txt".to_owned(),
        ))),
        CreateContainerComponent::Separator(CreateSeparator::new()),
        CreateContainerComponent::MediaGallery(CreateMediaGallery::new(gallery_items(1))),
    ]
}

fn v2_root_all_kinds() -> Vec<CreateComponent<'static>> {
    vec![
        CreateComponent::TextDisplay(text_display("root")),
        CreateComponent::Section(CreateSection::new(section_texts(1), thumbnail_accessory())),
        CreateComponent::ActionRow(CreateActionRow::buttons(nav_buttons(1))),
        CreateComponent::MediaGallery(CreateMediaGallery::new(gallery_items(1))),
        CreateComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
            "attachment://log.txt".to_owned(),
        ))),
        CreateComponent::Separator(CreateSeparator::new()),
        CreateComponent::Container(CreateContainer::new(container_texts(1))),
    ]
}

#[test]
fn action_row_allows_up_to_five_buttons() {
    check_action_row_children(&nav_buttons(5)).unwrap();
}

#[test]
fn action_row_rejects_a_sixth_button() {
    assert_eq!(
        check_action_row_children(&nav_buttons(6)).unwrap_err(),
        ChildRuleError::ActionRowTooManyButtons
    );
}

#[test]
fn action_row_rejects_an_empty_child_list() {
    assert_eq!(
        check_action_row_children(&[]).unwrap_err(),
        ChildRuleError::ActionRowEmpty
    );
}

#[test]
fn section_allows_three_text_displays_with_a_thumbnail() {
    check_section_children(&section_texts(3), Some(&thumbnail_accessory())).unwrap();
}

#[test]
fn section_allows_one_text_display_with_a_button_accessory() {
    check_section_children(
        &section_texts(1),
        Some(&CreateSectionAccessory::Button(section_button())),
    )
    .unwrap();
}

#[test]
fn section_rejects_a_fourth_text_display() {
    assert_eq!(
        check_section_children(&section_texts(4), Some(&thumbnail_accessory())).unwrap_err(),
        ChildRuleError::SectionTooManyTextDisplays
    );
}

#[test]
fn section_rejects_an_empty_child_list() {
    assert_eq!(
        check_section_children(&[], Some(&thumbnail_accessory())).unwrap_err(),
        ChildRuleError::SectionEmpty
    );
}

#[test]
fn section_rejects_a_missing_accessory() {
    assert_eq!(
        check_section_children(&section_texts(1), None).unwrap_err(),
        ChildRuleError::SectionMissingAccessory
    );
}

#[test]
fn select_menu_allows_up_to_twenty_five_options() {
    check_select_menu_options(&select_options(25)).unwrap();
}

#[test]
fn select_menu_rejects_a_twenty_sixth_option() {
    assert_eq!(
        check_select_menu_options(&select_options(26)).unwrap_err(),
        ChildRuleError::SelectMenuTooManyOptions
    );
}

#[test]
fn select_menu_allows_an_empty_option_list() {
    // The macro enforces no minimum option count, so neither does this check.
    check_select_menu_options(&[]).unwrap();
}

#[test]
fn media_gallery_allows_one_item() {
    check_media_gallery_items(&gallery_items(1)).unwrap();
}

#[test]
fn media_gallery_allows_ten_items() {
    check_media_gallery_items(&gallery_items(10)).unwrap();
}

#[test]
fn media_gallery_rejects_an_empty_item_list() {
    assert_eq!(
        check_media_gallery_items(&[]).unwrap_err(),
        ChildRuleError::MediaGalleryEmpty
    );
}

#[test]
fn media_gallery_rejects_an_eleventh_item() {
    assert_eq!(
        check_media_gallery_items(&gallery_items(11)).unwrap_err(),
        ChildRuleError::MediaGalleryTooManyItems
    );
}

#[test]
fn container_allows_every_child_kind_up_to_forty_children() {
    let mut children = container_all_kinds();
    children.extend(container_texts(34));
    assert_eq!(children.len(), 40);
    check_container_children(&children).unwrap();
}

#[test]
fn container_rejects_a_forty_first_child() {
    let mut children = container_all_kinds();
    children.extend(container_texts(35));
    assert_eq!(
        check_container_children(&children).unwrap_err(),
        ChildRuleError::ContainerTooManyChildren
    );
}

#[test]
fn container_allows_an_empty_child_list() {
    // The macro enforces no minimum container child count, so neither does
    // this check.
    check_container_children(&[]).unwrap();
}

#[test]
fn v2_root_allows_every_v2_child_kind() {
    check_v2_root_children(&v2_root_all_kinds()).unwrap();
}

#[test]
fn v2_root_allows_up_to_forty_children() {
    let mut children = v2_root_all_kinds();
    children.extend(root_texts(33));
    assert_eq!(children.len(), 40);
    check_v2_root_children(&children).unwrap();
}

#[test]
fn v2_root_rejects_a_forty_first_child() {
    let mut children = v2_root_all_kinds();
    children.extend(root_texts(34));
    assert_eq!(
        check_v2_root_children(&children).unwrap_err(),
        ChildRuleError::V2RootTooManyChildren
    );
}

#[test]
fn v2_root_rejects_an_empty_child_list() {
    assert_eq!(
        check_v2_root_children(&[]).unwrap_err(),
        ChildRuleError::V2RootEmpty
    );
}

#[test]
fn v2_root_rejects_a_modal_label_child() {
    let children = vec![
        CreateComponent::TextDisplay(text_display("root")),
        CreateComponent::Label(CreateLabel::select_menu(
            "Pick a region",
            string_select_menu(),
        )),
    ];
    let err = check_v2_root_children(&children).unwrap_err();
    assert!(
        err.to_string()
            .contains("`label` cannot appear inside `components_v2` — labels are modal-only")
    );
}
