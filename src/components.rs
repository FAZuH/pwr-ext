//! Wrapper types for serenity's message-component builders (v1 and v2).

use std::borrow::Cow;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;
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
use serenity::builder::CreateSelectMenu;
use serenity::builder::CreateSelectMenuKind;
use serenity::builder::CreateSelectMenuOption;
use serenity::builder::CreateSeparator;
use serenity::builder::CreateTextDisplay;
use serenity::builder::CreateThumbnail;
use serenity::builder::CreateUnfurledMediaItem;
use serenity::model::Colour;
use serenity::model::application::ButtonStyle;
use serenity::model::application::ComponentType;
use serenity::model::application::SeparatorSpacingSize;
use serenity::model::channel::ChannelType;
use serenity::model::channel::ReactionType;
use serenity::model::id::GenericChannelId;
use serenity::model::id::RoleId;
use serenity::model::id::SkuId;
use serenity::model::id::UserId;

use crate::util::capture_value;
use crate::util::mirror;
use crate::util::tag;

/// Mirror of [`CreateButton`].
///
/// The serialized shape is reproduced exactly; conversion goes through
/// upstream constructors (`new_link`, `new_premium`, `new`) so link/premium
/// buttons rebuild faithfully, then through setters that upstream deliberately
/// no-ops where they would contradict the constructor.
#[derive(Debug, Deserialize)]
pub struct CreateButtonDe<'a> {
    pub style: ButtonStyle,
    #[serde(rename = "type")]
    _kind: ComponentType,
    pub url: Option<Cow<'a, str>>,
    pub custom_id: Option<Cow<'a, str>>,
    pub sku_id: Option<SkuId>,
    pub label: Option<Cow<'a, str>>,
    pub emoji: Option<ReactionType>,
    #[serde(default)]
    pub disabled: bool,
}

impl<'a> From<CreateButtonDe<'a>> for CreateButton<'a> {
    fn from(de: CreateButtonDe<'a>) -> Self {
        let mut button = match (&de.url, de.sku_id) {
            (_, Some(sku_id)) => CreateButton::new_premium(sku_id),
            (Some(url), None) => CreateButton::new_link(url.clone()),
            (None, None) => CreateButton::new(de.custom_id.clone().unwrap_or_default()),
        };
        button = button.style(de.style);
        if let Some(custom_id) = de.custom_id {
            button = button.custom_id(custom_id);
        }
        if let Some(label) = de.label {
            button = button.label(label);
        }
        if let Some(emoji) = de.emoji {
            button = button.emoji(emoji);
        }
        button.disabled(de.disabled)
    }
}

/// Mirror of [`CreateActionRow`].
///
/// Upstream serializes both variants as `{"type": 1, "components": [...]}`;
/// the row kind is recovered by sniffing the numeric tag of the first child
/// (`2` = buttons, `3`/`5`/`6`/`7`/`8` = a single select menu).
#[derive(Debug)]
pub enum CreateActionRowDe<'a> {
    Buttons(Vec<CreateButtonDe<'a>>),
    SelectMenu(CreateSelectMenuDe<'a>),
}

impl<'a> From<CreateActionRowDe<'a>> for CreateActionRow<'a> {
    fn from(de: CreateActionRowDe<'a>) -> Self {
        match de {
            CreateActionRowDe::Buttons(buttons) => CreateActionRow::buttons(
                buttons
                    .into_iter()
                    .map(CreateButton::from)
                    .collect::<Vec<_>>(),
            ),
            CreateActionRowDe::SelectMenu(menu) => CreateActionRow::select_menu(menu),
        }
    }
}

impl<'de, 'a> Deserialize<'de> for CreateActionRowDe<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        parse_action_row(&value).map_err(D::Error::custom)
    }
}

fn parse_action_row(value: &Value) -> Result<CreateActionRowDe<'static>, String> {
    if tag(value)? != 1 {
        return Err(format!("expected an action row, got {value}"));
    }
    let components = value
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("action row missing \"components\" array: {value}"))?;

    let Some(first) = components.first() else {
        return Ok(CreateActionRowDe::Buttons(Vec::new()));
    };

    match tag(first)? {
        // ComponentType::BUTTON
        2 => {
            let buttons = components
                .iter()
                .map(|component| -> Result<CreateButtonDe<'static>, String> {
                    if tag(component)? != 2 {
                        return Err(format!(
                            "cannot mix component types in one action row: {component}"
                        ));
                    }
                    mirror(component)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CreateActionRowDe::Buttons(buttons))
        }
        kind @ (3 | 5 | 6 | 7 | 8) => {
            if components.len() != 1 {
                return Err(format!(
                    "a select-menu action row holds exactly one component, got {}: {value}",
                    components.len()
                ));
            }
            let menu: CreateSelectMenuDe<'static> = mirror(first)?;
            debug_assert_eq!(menu.kind_number(), kind);
            Ok(CreateActionRowDe::SelectMenu(menu))
        }
        other => Err(format!(
            "unsupported action-row component type {other}: {first}"
        )),
    }
}

/// Mirror of [`CreateSelectMenu`].
///
/// The flattened per-kind fields (`options`, `channel_types`,
/// `default_values`) are resolved into the matching
/// [`CreateSelectMenuKind`] during deserialization; unknown kind numbers and
/// missing required fields error there.
#[derive(Debug)]
pub struct CreateSelectMenuDe<'a> {
    pub custom_id: Cow<'a, str>,
    pub placeholder: Option<Cow<'a, str>>,
    pub min_values: Option<u8>,
    pub max_values: Option<u8>,
    pub required: Option<bool>,
    pub disabled: Option<bool>,
    kind: SelectKindDe<'a>,
}

impl CreateSelectMenuDe<'_> {
    /// The numeric `"type"` the menu was parsed from.
    pub fn kind_number(&self) -> u64 {
        self.kind.number()
    }
}

impl<'a> From<CreateSelectMenuDe<'a>> for CreateSelectMenu<'a> {
    fn from(de: CreateSelectMenuDe<'a>) -> Self {
        let mut menu = CreateSelectMenu::new(de.custom_id, de.kind.into_select_menu_kind());
        if let Some(placeholder) = de.placeholder {
            menu = menu.placeholder(placeholder);
        }
        if let Some(min_values) = de.min_values {
            menu = menu.min_values(min_values);
        }
        if let Some(max_values) = de.max_values {
            menu = menu.max_values(max_values);
        }
        if let Some(required) = de.required {
            menu = menu.required(required);
        }
        if let Some(disabled) = de.disabled {
            menu = menu.disabled(disabled);
        }
        menu
    }
}

#[derive(Debug)]
enum SelectKindDe<'a> {
    String {
        options: Vec<CreateSelectMenuOptionDe<'a>>,
    },
    User {
        default_ids: Vec<u64>,
    },
    Role {
        default_ids: Vec<u64>,
    },
    Mentionable {
        user_ids: Vec<u64>,
        role_ids: Vec<u64>,
    },
    Channel {
        channel_types: Option<Vec<ChannelType>>,
        default_ids: Vec<u64>,
    },
}

impl<'a> SelectKindDe<'a> {
    fn number(&self) -> u64 {
        match self {
            SelectKindDe::String { .. } => 3,
            SelectKindDe::User { .. } => 5,
            SelectKindDe::Role { .. } => 6,
            SelectKindDe::Mentionable { .. } => 7,
            SelectKindDe::Channel { .. } => 8,
        }
    }

    fn into_select_menu_kind(self) -> CreateSelectMenuKind<'a> {
        match self {
            SelectKindDe::String { options } => CreateSelectMenuKind::String {
                options: Cow::Owned(
                    options
                        .into_iter()
                        .map(CreateSelectMenuOption::from)
                        .collect(),
                ),
            },
            SelectKindDe::User { default_ids } => CreateSelectMenuKind::User {
                default_users: owned_ids(default_ids).map(Cow::Owned),
            },
            SelectKindDe::Role { default_ids } => CreateSelectMenuKind::Role {
                default_roles: owned_roles(default_ids).map(Cow::Owned),
            },
            SelectKindDe::Mentionable { user_ids, role_ids } => CreateSelectMenuKind::Mentionable {
                default_users: owned_ids(user_ids).map(Cow::Owned),
                default_roles: owned_roles(role_ids).map(Cow::Owned),
            },
            SelectKindDe::Channel {
                channel_types,
                default_ids,
            } => CreateSelectMenuKind::Channel {
                channel_types: channel_types.map(Cow::Owned),
                default_channels: non_empty(
                    default_ids.into_iter().map(GenericChannelId::new).collect(),
                )
                .map(Cow::Owned),
            },
        }
    }
}

fn owned_ids(ids: Vec<u64>) -> Option<Vec<UserId>> {
    non_empty(ids.into_iter().map(UserId::new).collect())
}

fn owned_roles(ids: Vec<u64>) -> Option<Vec<RoleId>> {
    non_empty(ids.into_iter().map(RoleId::new).collect())
}

fn non_empty<T>(items: Vec<T>) -> Option<Vec<T>> {
    if items.is_empty() { None } else { Some(items) }
}

/// Mirror of [`CreateSelectMenuOption`](serenity::builder::CreateSelectMenuOption).
#[derive(Clone, Debug, Deserialize)]
pub struct CreateSelectMenuOptionDe<'a> {
    pub label: Cow<'a, str>,
    pub value: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub emoji: Option<ReactionType>,
    pub default: Option<bool>,
}

impl<'a> From<CreateSelectMenuOptionDe<'a>> for CreateSelectMenuOption<'a> {
    fn from(de: CreateSelectMenuOptionDe<'a>) -> Self {
        let mut option = CreateSelectMenuOption::new(de.label, de.value);
        if let Some(description) = de.description {
            option = option.description(description);
        }
        if let Some(emoji) = de.emoji {
            option = option.emoji(emoji);
        }
        if let Some(default) = de.default {
            option = option.default_selection(default);
        }
        option
    }
}

/// One entry of a select menu's `default_values` array.
#[derive(Debug, Deserialize)]
struct SelectDefaultDe {
    id: GenericChannelId,
    #[serde(rename = "type")]
    target: String,
}

#[derive(Debug, Deserialize)]
struct RawSelectMenuDe<'a> {
    custom_id: Cow<'a, str>,
    #[serde(default)]
    placeholder: Option<Cow<'a, str>>,
    #[serde(default)]
    min_values: Option<u8>,
    #[serde(default)]
    max_values: Option<u8>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    disabled: Option<bool>,
    #[serde(rename = "type")]
    _kind: u64,
    #[serde(default)]
    options: Option<Vec<CreateSelectMenuOptionDe<'a>>>,
    #[serde(default)]
    channel_types: Option<Vec<ChannelType>>,
    #[serde(default)]
    default_values: Vec<SelectDefaultDe>,
}

impl<'de, 'a> Deserialize<'de> for CreateSelectMenuDe<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        let raw: RawSelectMenuDe<'static> = mirror(&value).map_err(D::Error::custom)?;
        let kind = resolve_kind(&raw).map_err(D::Error::custom)?;

        Ok(CreateSelectMenuDe {
            custom_id: raw.custom_id,
            placeholder: raw.placeholder,
            min_values: raw.min_values,
            max_values: raw.max_values,
            required: raw.required,
            disabled: raw.disabled,
            kind,
        })
    }
}

fn resolve_kind<'a>(raw: &RawSelectMenuDe<'a>) -> Result<SelectKindDe<'a>, String> {
    check_default_targets(&raw.default_values)?;

    match raw._kind {
        3 => {
            let options = raw
                .options
                .as_ref()
                .ok_or_else(|| "string select requires \"options\"".to_string())?
                .clone();
            Ok(SelectKindDe::String { options })
        }
        5 => Ok(SelectKindDe::User {
            default_ids: ids_of(&raw.default_values, "user"),
        }),
        6 => Ok(SelectKindDe::Role {
            default_ids: ids_of(&raw.default_values, "role"),
        }),
        7 => Ok(SelectKindDe::Mentionable {
            user_ids: ids_of(&raw.default_values, "user"),
            role_ids: ids_of(&raw.default_values, "role"),
        }),
        8 => Ok(SelectKindDe::Channel {
            channel_types: raw.channel_types.clone(),
            default_ids: ids_of(&raw.default_values, "channel"),
        }),
        other => Err(format!("unsupported select-menu type {other}")),
    }
}

fn check_default_targets(defaults: &[SelectDefaultDe]) -> Result<(), String> {
    for default in defaults {
        match default.target.as_str() {
            "user" | "role" | "channel" => {}
            other => {
                return Err(format!(
                    "unknown select-menu default type \"{other}\" for id {}",
                    default.id
                ));
            }
        }
    }
    Ok(())
}

fn ids_of(defaults: &[SelectDefaultDe], target: &str) -> Vec<u64> {
    defaults
        .iter()
        .filter(|default| default.target == target)
        .map(|default| default.id.get())
        .collect()
}

/// Mirror of the upstream-untagged [`CreateComponent`] enum.
///
/// Dispatch is by explicit numeric `"type"` tag over an intermediate
/// [`serde_json::Value`](serde_json::Value); the result is always owned
/// (`'static`). See the crate docs for why `#[serde(untagged)]` must not be
/// used here.
#[derive(Debug)]
pub struct CreateComponentDe(pub CreateComponent<'static>);

impl<'de> Deserialize<'de> for CreateComponentDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        parse_component(&value)
            .map_err(D::Error::custom)
            .map(CreateComponentDe)
    }
}

impl From<CreateComponentDe> for CreateComponent<'static> {
    fn from(de: CreateComponentDe) -> Self {
        de.0
    }
}

fn parse_component(value: &Value) -> Result<CreateComponent<'static>, String> {
    Ok(match tag(value)? {
        1 => CreateComponent::ActionRow(parse_action_row(value)?.into()),
        9 => CreateComponent::Section(parse_section(value)?),
        10 => CreateComponent::TextDisplay(parse_text_display(value)?),
        12 => CreateComponent::MediaGallery(parse_media_gallery(value)?),
        13 => CreateComponent::File(parse_file(value)?),
        14 => CreateComponent::Separator(parse_separator(value)?),
        17 => CreateComponent::Container(parse_container(value)?),
        other => return Err(format!("unsupported component type {other}: {value}")),
    })
}

// Same tree, but yielding the container-child variant of each node.
fn parse_container_component(value: &Value) -> Result<CreateContainerComponent<'static>, String> {
    Ok(match tag(value)? {
        1 => CreateContainerComponent::ActionRow(parse_action_row(value)?.into()),
        9 => CreateContainerComponent::Section(parse_section(value)?),
        10 => CreateContainerComponent::TextDisplay(parse_text_display(value)?),
        12 => CreateContainerComponent::MediaGallery(parse_media_gallery(value)?),
        13 => CreateContainerComponent::File(parse_file(value)?),
        14 => CreateContainerComponent::Separator(parse_separator(value)?),
        other => {
            return Err(format!(
                "unsupported container component type {other}: {value}"
            ));
        }
    })
}

#[derive(Debug, Deserialize)]
struct TextDisplayDe<'a> {
    #[serde(rename = "type")]
    _kind: u8,
    content: Cow<'a, str>,
}

pub(crate) fn parse_text_display(value: &Value) -> Result<CreateTextDisplay<'static>, String> {
    let de: TextDisplayDe<'static> = mirror(value)?;
    Ok(CreateTextDisplay::new(de.content))
}
#[derive(Debug, Deserialize)]
struct SectionDe<'a> {
    #[serde(rename = "type")]
    _kind: u8,
    #[serde(default)]
    components: Vec<TextDisplayDe<'a>>,
    accessory: Value,
}

fn parse_section(value: &Value) -> Result<CreateSection<'static>, String> {
    let de: SectionDe<'static> = mirror(value)?;
    let texts = de
        .components
        .into_iter()
        .map(|text| CreateSectionComponent::TextDisplay(CreateTextDisplay::new(text.content)))
        .collect::<Vec<_>>();
    let accessory = parse_accessory(&de.accessory)?;
    Ok(CreateSection::new(texts, accessory))
}

// Discriminated by "type": 11 = thumbnail, 2 = button.
fn parse_accessory(value: &Value) -> Result<CreateSectionAccessory<'static>, String> {
    Ok(match tag(value)? {
        11 => CreateSectionAccessory::Thumbnail(parse_thumbnail(value)?),
        2 => CreateSectionAccessory::Button(mirror::<CreateButtonDe<'static>>(value)?.into()),
        other => {
            return Err(format!(
                "unsupported section accessory type {other}: {value}"
            ));
        }
    })
}

#[derive(Debug, Deserialize)]
struct ThumbnailDe<'a> {
    media: UnfurledMediaDe<'a>,
    #[serde(default)]
    description: Option<Cow<'a, str>>,
    #[serde(default)]
    spoiler: Option<bool>,
}

fn parse_thumbnail(value: &Value) -> Result<CreateThumbnail<'static>, String> {
    let de: ThumbnailDe<'static> = mirror(value)?;
    let mut thumbnail = CreateThumbnail::new(CreateUnfurledMediaItem::new(de.media.url));
    if let Some(description) = de.description {
        thumbnail = thumbnail.description(description);
    }
    if let Some(spoiler) = de.spoiler {
        thumbnail = thumbnail.spoiler(spoiler);
    }
    Ok(thumbnail)
}

#[derive(Debug, Deserialize)]
struct UnfurledMediaDe<'a> {
    url: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
struct MediaGalleryDe<'a> {
    items: Vec<MediaGalleryItemDe<'a>>,
}

fn parse_media_gallery(value: &Value) -> Result<CreateMediaGallery<'static>, String> {
    let de: MediaGalleryDe<'static> = mirror(value)?;
    let items = de
        .items
        .into_iter()
        .map(CreateMediaGalleryItem::from)
        .collect::<Vec<_>>();
    Ok(CreateMediaGallery::new(items))
}

#[derive(Debug, Deserialize)]
struct MediaGalleryItemDe<'a> {
    media: UnfurledMediaDe<'a>,
    #[serde(default)]
    description: Option<Cow<'a, str>>,
    #[serde(default)]
    spoiler: Option<bool>,
}

impl From<MediaGalleryItemDe<'static>> for CreateMediaGalleryItem<'static> {
    fn from(de: MediaGalleryItemDe<'static>) -> Self {
        let mut item = CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(de.media.url));
        if let Some(description) = de.description {
            item = item.description(description);
        }
        if let Some(spoiler) = de.spoiler {
            item = item.spoiler(spoiler);
        }
        item
    }
}

#[derive(Debug, Deserialize)]
struct FileDe<'a> {
    file: UnfurledMediaDe<'a>,
    #[serde(default)]
    spoiler: Option<bool>,
}

fn parse_file(value: &Value) -> Result<CreateFile<'static>, String> {
    let de: FileDe<'static> = mirror(value)?;
    let mut file = CreateFile::new(CreateUnfurledMediaItem::new(de.file.url));
    if let Some(spoiler) = de.spoiler {
        file = file.spoiler(spoiler);
    }
    Ok(file)
}

#[derive(Debug, Deserialize)]
struct SeparatorDe {
    divider: Option<bool>,
    spacing: Option<SeparatorSpacingSize>,
}

fn parse_separator(value: &Value) -> Result<CreateSeparator, String> {
    let de: SeparatorDe = mirror(value)?;
    let mut separator = CreateSeparator::new();
    if let Some(divider) = de.divider {
        separator = separator.divider(divider);
    }
    if let Some(spacing) = de.spacing {
        separator = separator.spacing(spacing);
    }
    Ok(separator)
}

#[derive(Debug, Deserialize)]
struct ContainerDe {
    #[serde(default)]
    accent_color: Option<Colour>,
    #[serde(default)]
    spoiler: Option<bool>,
}

fn parse_container(value: &Value) -> Result<CreateContainer<'static>, String> {
    let raw_components = value
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("container missing \"components\" array: {value}"))?;
    let components = raw_components
        .iter()
        .map(parse_container_component)
        .collect::<Result<Vec<_>, _>>()?;

    let de: ContainerDe = mirror(value)?;
    let mut container = CreateContainer::new(components);
    if let Some(accent_color) = de.accent_color {
        container = container.accent_colour(accent_color);
    }
    if let Some(spoiler) = de.spoiler {
        container = container.spoiler(spoiler);
    }
    Ok(container)
}
