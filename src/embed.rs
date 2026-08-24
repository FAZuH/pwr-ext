//! Wrapper types for serenity's embed builders.

use std::borrow::Cow;

use serde::Deserialize;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateEmbedFooter;
use serenity::model::Colour;
use serenity::model::Timestamp;

/// Mirror of [`CreateEmbed`].
///
/// The `type` field is captured but not enforced: upstream has no setter for
/// it and always rebuilds as `"rich"` (spec D7).
#[derive(Debug, Deserialize)]
pub struct CreateEmbedDe<'a> {
    pub title: Option<Cow<'a, str>>,
    #[serde(rename = "type")]
    _kind: Option<String>,
    pub description: Option<Cow<'a, str>>,
    pub url: Option<Cow<'a, str>>,
    pub timestamp: Option<Timestamp>,
    #[serde(rename = "color")]
    pub colour: Option<Colour>,
    pub footer: Option<CreateEmbedFooterDe<'a>>,
    pub image: Option<CreateEmbedImageDe<'a>>,
    pub thumbnail: Option<CreateEmbedImageDe<'a>>,
    pub author: Option<CreateEmbedAuthorDe<'a>>,
    #[serde(default)]
    pub fields: Vec<CreateEmbedFieldDe<'a>>,
}

/// Mirror of [`CreateEmbedAuthor`].
///
/// Upstream serializes `url`/`icon_url` as explicit `null` when unset (no
/// skip attribute), so they stay plain `Option`s.
#[derive(Debug, Deserialize)]
pub struct CreateEmbedAuthorDe<'a> {
    pub name: Cow<'a, str>,
    pub url: Option<Cow<'a, str>>,
    pub icon_url: Option<Cow<'a, str>>,
}

/// Mirror of [`CreateEmbedFooter`].
#[derive(Debug, Deserialize)]
pub struct CreateEmbedFooterDe<'a> {
    pub text: Cow<'a, str>,
    pub icon_url: Option<Cow<'a, str>>,
}

/// Mirror of the upstream-private `CreateEmbedField`.
///
/// `inline` is always serialized by upstream and therefore required here.
#[derive(Debug, Deserialize)]
pub struct CreateEmbedFieldDe<'a> {
    pub name: Cow<'a, str>,
    pub value: Cow<'a, str>,
    pub inline: bool,
}

/// Mirror of the upstream-private `CreateEmbedImage`.
#[derive(Debug, Deserialize)]
pub struct CreateEmbedImageDe<'a> {
    pub url: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
}

impl<'a> From<CreateEmbedDe<'a>> for CreateEmbed<'a> {
    fn from(de: CreateEmbedDe<'a>) -> Self {
        // CreateEmbed::new() is Default, which pins `type` to "rich".
        let mut embed = CreateEmbed::new();
        if let Some(title) = de.title {
            embed = embed.title(title);
        }
        if let Some(description) = de.description {
            embed = embed.description(description);
        }
        if let Some(url) = de.url {
            embed = embed.url(url);
        }
        if let Some(timestamp) = de.timestamp {
            embed = embed.timestamp(timestamp);
        }
        if let Some(colour) = de.colour {
            embed = embed.colour(colour);
        }
        if let Some(footer) = de.footer {
            embed = embed.footer(footer.into());
        }
        if let Some(image) = de.image {
            embed = embed.image(image.url, image.description);
        }
        if let Some(thumbnail) = de.thumbnail {
            embed = embed.thumbnail(thumbnail.url, thumbnail.description);
        }
        if let Some(author) = de.author {
            embed = embed.author(author.into());
        }
        if !de.fields.is_empty() {
            embed = embed.fields(de.fields.into_iter().map(|f| (f.name, f.value, f.inline)));
        }
        embed
    }
}

impl<'a> From<CreateEmbedAuthorDe<'a>> for CreateEmbedAuthor<'a> {
    fn from(de: CreateEmbedAuthorDe<'a>) -> Self {
        let mut author = CreateEmbedAuthor::new(de.name);
        if let Some(url) = de.url {
            author = author.url(url);
        }
        if let Some(icon_url) = de.icon_url {
            author = author.icon_url(icon_url);
        }
        author
    }
}

impl<'a> From<CreateEmbedFooterDe<'a>> for CreateEmbedFooter<'a> {
    fn from(de: CreateEmbedFooterDe<'a>) -> Self {
        let mut footer = CreateEmbedFooter::new(de.text);
        if let Some(icon_url) = de.icon_url {
            footer = footer.icon_url(icon_url);
        }
        footer
    }
}
