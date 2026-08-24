//! Wrapper types for serenity's modal builders.

use std::borrow::Cow;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;
use serenity::builder::CreateCheckbox;
use serenity::builder::CreateCheckboxGroup;
use serenity::builder::CreateCheckboxGroupOption;
use serenity::builder::CreateFileUpload;
use serenity::builder::CreateInputText;
use serenity::builder::CreateLabel;
use serenity::builder::CreateModal;
use serenity::builder::CreateModalComponent;
use serenity::builder::CreateRadioGroup;
use serenity::builder::CreateRadioGroupOption;
use serenity::builder::CreateSelectMenu;
use serenity::model::application::InputTextStyle;

use crate::components::CreateSelectMenuDe;
use crate::components::parse_text_display;
use crate::util::capture_value;
use crate::util::mirror;
use crate::util::tag;

/// Mirror of [`CreateModal`].
///
/// Lifetime-free like the dispatched trees: its components are a tagged
/// union, so the whole mirror follows the owned-tree convention.
#[derive(Debug, Deserialize)]
pub struct CreateModalDe {
    #[serde(default)]
    components: Vec<CreateModalComponentDe>,
    custom_id: Cow<'static, str>,
    title: Cow<'static, str>,
}

impl From<CreateModalDe> for CreateModal<'static> {
    fn from(de: CreateModalDe) -> Self {
        CreateModal::new(de.custom_id, de.title).components(
            de.components
                .into_iter()
                .map(CreateModalComponent::from)
                .collect::<Vec<_>>(),
        )
    }
}

/// Mirror of the upstream-untagged [`CreateModalComponent`] enum.
///
/// Dispatch is by explicit numeric `"type"` tag (10 = text display,
/// 18 = label); see the crate docs for why `#[serde(untagged)]` must not be
/// used here.
#[derive(Debug)]
pub struct CreateModalComponentDe(pub CreateModalComponent<'static>);

impl From<CreateModalComponentDe> for CreateModalComponent<'static> {
    fn from(de: CreateModalComponentDe) -> Self {
        de.0
    }
}

impl<'de> Deserialize<'de> for CreateModalComponentDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        parse_modal_component(&value)
            .map_err(D::Error::custom)
            .map(CreateModalComponentDe)
    }
}

fn parse_modal_component(value: &Value) -> Result<CreateModalComponent<'static>, String> {
    Ok(match tag(value)? {
        10 => CreateModalComponent::TextDisplay(parse_text_display(value)?),
        18 => CreateModalComponent::Label(parse_label(value)?),
        other => {
            return Err(format!("unsupported modal component type {other}: {value}"));
        }
    })
}

/// Mirror of [`CreateLabel`].
///
/// Opaque wrapper around the rebuilt label; the label's inner component is
/// itself a tagged union resolved during deserialization.
#[derive(Debug)]
pub struct CreateLabelDe(pub CreateLabel<'static>);

impl From<CreateLabelDe> for CreateLabel<'static> {
    fn from(de: CreateLabelDe) -> Self {
        de.0
    }
}

impl<'de> Deserialize<'de> for CreateLabelDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        parse_label(&value)
            .map_err(D::Error::custom)
            .map(CreateLabelDe)
    }
}

#[derive(Debug, Deserialize)]
struct RawLabelDe {
    #[serde(rename = "type")]
    _kind: u8,
    label: Cow<'static, str>,
    #[serde(default)]
    description: Option<Cow<'static, str>>,
    component: Value,
}

fn parse_label(value: &Value) -> Result<CreateLabel<'static>, String> {
    let de: RawLabelDe = mirror(value)?;
    let child = parse_label_component(&de.component)?;

    let mut label = match child {
        LabelChildDe::SelectMenu(menu) => CreateLabel::select_menu(de.label, menu),
        LabelChildDe::InputText(input) => CreateLabel::input_text(de.label, input),
        LabelChildDe::FileUpload(upload) => CreateLabel::file_upload(de.label, upload),
        LabelChildDe::RadioGroup(group) => CreateLabel::radio_group(de.label, group),
        LabelChildDe::CheckboxGroup(group) => CreateLabel::checkbox_group(de.label, group),
        LabelChildDe::Checkbox(checkbox) => CreateLabel::checkbox(de.label, checkbox),
    };
    if let Some(description) = de.description {
        label = label.description(description);
    }
    Ok(label)
}

// The label's inner component, discriminated by "type": select menus
// 3/5/6/7/8, text input 4, file upload 19, radio group 21,
// checkbox group 22, checkbox 23.
enum LabelChildDe {
    SelectMenu(CreateSelectMenu<'static>),
    InputText(CreateInputText<'static>),
    FileUpload(CreateFileUpload<'static>),
    RadioGroup(CreateRadioGroup<'static>),
    CheckboxGroup(CreateCheckboxGroup<'static>),
    Checkbox(CreateCheckbox<'static>),
}

fn parse_label_component(value: &Value) -> Result<LabelChildDe, String> {
    Ok(match tag(value)? {
        kind @ (3 | 5 | 6 | 7 | 8) => {
            let menu: CreateSelectMenuDe<'static> = mirror(value)?;
            debug_assert_eq!(menu.kind_number(), kind);
            LabelChildDe::SelectMenu(menu.into())
        }
        4 => LabelChildDe::InputText(parse_input_text(value)?),
        19 => LabelChildDe::FileUpload(parse_file_upload(value)?),
        21 => LabelChildDe::RadioGroup(parse_radio_group(value)?),
        22 => LabelChildDe::CheckboxGroup(parse_checkbox_group(value)?),
        23 => LabelChildDe::Checkbox(parse_checkbox(value)?),
        other => {
            return Err(format!("unsupported label component type {other}: {value}"));
        }
    })
}

// `min_length`/`max_length` have no skip attribute upstream and serialize as
// explicit `null` when unset; plain `Option`s reproduce that.
#[derive(Debug, Deserialize)]
struct InputTextDe {
    #[serde(rename = "type")]
    _kind: u8,
    custom_id: Cow<'static, str>,
    style: InputTextStyle,
    min_length: Option<u16>,
    max_length: Option<u16>,
    required: bool,
    #[serde(default)]
    value: Option<Cow<'static, str>>,
    #[serde(default)]
    placeholder: Option<Cow<'static, str>>,
}

fn parse_input_text(value: &Value) -> Result<CreateInputText<'static>, String> {
    let de: InputTextDe = mirror(value)?;
    let mut input = CreateInputText::new(de.style, de.custom_id);
    if let Some(min_length) = de.min_length {
        input = input.min_length(min_length);
    }
    if let Some(max_length) = de.max_length {
        input = input.max_length(max_length);
    }
    input = input.required(de.required);
    if let Some(value) = de.value {
        input = input.value(value);
    }
    if let Some(placeholder) = de.placeholder {
        input = input.placeholder(placeholder);
    }
    Ok(input)
}

// `min_values`, `max_values`, and `required` are always serialized; only
// `file_types` is skipped when empty.
#[derive(Debug, Deserialize)]
struct FileUploadDe {
    #[serde(rename = "type")]
    _kind: u8,
    custom_id: Cow<'static, str>,
    min_values: u8,
    max_values: u8,
    required: bool,
    #[serde(default)]
    file_types: Vec<Cow<'static, str>>,
}

fn parse_file_upload(value: &Value) -> Result<CreateFileUpload<'static>, String> {
    let de: FileUploadDe = mirror(value)?;
    let upload = CreateFileUpload::new(de.custom_id)
        .min_values(de.min_values)
        .max_values(de.max_values)
        .required(de.required);
    if de.file_types.is_empty() {
        Ok(upload)
    } else {
        Ok(upload.file_types(de.file_types))
    }
}

#[derive(Debug, Deserialize)]
struct RadioGroupDe {
    #[serde(rename = "type")]
    _kind: u8,
    custom_id: Cow<'static, str>,
    options: Vec<RadioGroupOptionDe>,
    #[serde(default)]
    required: Option<bool>,
}

fn parse_radio_group(value: &Value) -> Result<CreateRadioGroup<'static>, String> {
    let de: RadioGroupDe = mirror(value)?;
    let options = de
        .options
        .into_iter()
        .map(CreateRadioGroupOption::from)
        .collect::<Vec<_>>();
    let mut group = CreateRadioGroup::new(de.custom_id, options);
    if let Some(required) = de.required {
        group = group.required(required);
    }
    Ok(group)
}

impl From<RadioGroupOptionDe> for CreateRadioGroupOption<'static> {
    fn from(de: RadioGroupOptionDe) -> Self {
        let mut option = CreateRadioGroupOption::new(de.label, de.value);
        if let Some(description) = de.description {
            option = option.description(description);
        }
        if let Some(default) = de.default {
            option = option.default_selection(default);
        }
        option
    }
}

#[derive(Debug, Deserialize)]
struct RadioGroupOptionDe {
    label: Cow<'static, str>,
    value: Cow<'static, str>,
    #[serde(default)]
    description: Option<Cow<'static, str>>,
    #[serde(default)]
    default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CheckboxGroupDe {
    #[serde(rename = "type")]
    _kind: u8,
    custom_id: Cow<'static, str>,
    options: Vec<CheckboxGroupOptionDe>,
    min_values: Option<u8>,
    max_values: Option<u8>,
    required: Option<bool>,
}

fn parse_checkbox_group(value: &Value) -> Result<CreateCheckboxGroup<'static>, String> {
    let de: CheckboxGroupDe = mirror(value)?;
    let options = de
        .options
        .into_iter()
        .map(CheckboxGroupOptionDe::into_builder)
        .collect::<Vec<_>>();

    // Upstream couples these two setters: `min_values(0)` force-enables
    // `required = false`, and `required(true)` bumps a zero `min_values` to
    // one. Applying them in builder order (min, max, required) reproduces
    // upstream semantics exactly.
    let mut group = CreateCheckboxGroup::new(de.custom_id, options);
    if let Some(min_values) = de.min_values {
        group = group.min_values(min_values);
    }
    if let Some(max_values) = de.max_values {
        group = group.max_values(max_values);
    }
    if let Some(required) = de.required {
        group = group.required(required);
    }
    Ok(group)
}

#[derive(Debug, Deserialize)]
struct CheckboxGroupOptionDe {
    label: Cow<'static, str>,
    value: Cow<'static, str>,
    #[serde(default)]
    description: Option<Cow<'static, str>>,
    #[serde(default)]
    default: Option<bool>,
}

impl CheckboxGroupOptionDe {
    fn into_builder(self) -> CreateCheckboxGroupOption<'static> {
        let mut option = CreateCheckboxGroupOption::new(self.label, self.value);
        if let Some(description) = self.description {
            option = option.description(description);
        }
        if let Some(default) = self.default {
            option = option.default_selection(default);
        }
        option
    }
}

#[derive(Debug, Deserialize)]
struct CheckboxDe {
    #[serde(rename = "type")]
    _kind: u8,
    custom_id: Cow<'static, str>,
    #[serde(default)]
    default: Option<bool>,
}

fn parse_checkbox(value: &Value) -> Result<CreateCheckbox<'static>, String> {
    let de: CheckboxDe = mirror(value)?;
    let mut checkbox = CreateCheckbox::new(de.custom_id);
    if let Some(default) = de.default {
        checkbox = checkbox.default_selected(default);
    }
    Ok(checkbox)
}
