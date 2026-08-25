//! Wrapper types for remaining JSON-emitting serenity builders that do not
//! fit another module: welcome-screen channels, role colours, and test
//! entitlements.

use std::borrow::Cow;
use std::convert::TryInto as _;

use serde::Deserialize;
use serde_json::Value;
use serenity::builder::CreateGuildWelcomeChannel;
use serenity::builder::CreateRoleColours;
use serenity::builder::CreateTestEntitlement;
use serenity::model::Colour;
use serenity::model::guild::GuildWelcomeChannelEmoji;
use serenity::model::id::EmojiId;
use serenity::model::id::GenericChannelId;
use small_fixed_array::FixedString as WelcomeScreenFixedString;

use crate::util::mirror;
use crate::util::opaque_wrapper;

/// Mirror of [`CreateGuildWelcomeChannel`].
///
/// All four fields serialize unconditionally upstream, so unset emojis
/// rebuild as explicit `null`s. The emoji setter takes a two-variant enum
/// whose custom variant always carries both id and name, so a payload with
/// `emoji_id` but no `emoji_name` errors — the builder cannot represent it.
#[derive(Debug)]
pub struct CreateGuildWelcomeChannelDe(CreateGuildWelcomeChannel<'static>);

opaque_wrapper!(
    CreateGuildWelcomeChannelDe,
    CreateGuildWelcomeChannel<'static>,
    parse_welcome_channel
);

#[derive(Debug, Deserialize)]
struct RawWelcomeChannelDe {
    channel_id: GenericChannelId,
    #[serde(default)]
    emoji_name: Option<Cow<'static, str>>,
    #[serde(default)]
    emoji_id: Option<EmojiId>,
    description: Cow<'static, str>,
}

fn parse_welcome_channel(value: &Value) -> Result<CreateGuildWelcomeChannel<'static>, String> {
    let de: RawWelcomeChannelDe = mirror(value)?;
    let mut channel = CreateGuildWelcomeChannel::new(de.channel_id, de.description);

    match (de.emoji_id, de.emoji_name) {
        (None, None) => {}
        (Some(emoji_id), Some(emoji_name)) => {
            let name = fixed_emoji_name(emoji_name)?;
            channel = channel.emoji(GuildWelcomeChannelEmoji::Custom { id: emoji_id, name });
        }
        (None, Some(emoji_name)) => {
            let name = fixed_emoji_name(emoji_name)?;
            channel = channel.emoji(GuildWelcomeChannelEmoji::Unicode(name));
        }
        (Some(_), None) => {
            return Err(
                "a welcome channel cannot carry an emoji id without an emoji name".to_string(),
            );
        }
    }
    Ok(channel)
}

fn fixed_emoji_name(name: Cow<'static, str>) -> Result<WelcomeScreenFixedString, String> {
    name.into_owned()
        .try_into()
        .map_err(|_| "emoji name is not representable".to_string())
}

/// Mirror of [`CreateRoleColours`].
///
/// `primary_color` is required; the optional colours have skip attributes
/// upstream and stay absent unless set.
#[derive(Debug, Deserialize)]
pub struct CreateRoleColoursDe {
    primary_color: Colour,
    secondary_color: Option<Colour>,
    tertiary_color: Option<Colour>,
}

impl From<CreateRoleColoursDe> for CreateRoleColours {
    fn from(de: CreateRoleColoursDe) -> Self {
        let colours = CreateRoleColours::new(de.primary_color);
        match (de.secondary_color, de.tertiary_color) {
            (Some(secondary), Some(tertiary)) => colours
                .secondary_colour(secondary)
                .tertiary_colour(tertiary),
            (Some(secondary), None) => colours.secondary_colour(secondary),
            (None, Some(tertiary)) => colours.tertiary_colour(tertiary),
            (None, None) => colours,
        }
    }
}

/// Mirror of [`CreateTestEntitlement`].
///
/// Upstream builds `owner_type` from a two-variant owner enum (1 = guild,
/// 2 = user), so any other number errors at deserialization time.
#[derive(Debug)]
pub struct CreateTestEntitlementDe(CreateTestEntitlement);

opaque_wrapper!(
    CreateTestEntitlementDe,
    CreateTestEntitlement,
    parse_test_entitlement
);

#[derive(Debug, Deserialize)]
struct RawTestEntitlementDe {
    sku_id: serenity::model::id::SkuId,
    owner_id: serenity::model::id::GenericId,
    owner_type: u8,
}

fn parse_test_entitlement(value: &Value) -> Result<CreateTestEntitlement, String> {
    use serenity::builder::EntitlementOwner;

    let de: RawTestEntitlementDe = mirror(value)?;
    let owner = match de.owner_type {
        1 => EntitlementOwner::Guild(serenity::model::id::GuildId::new(de.owner_id.get())),
        2 => EntitlementOwner::User(serenity::model::id::UserId::new(de.owner_id.get())),
        other => return Err(format!("unsupported entitlement owner type {other}")),
    };
    Ok(CreateTestEntitlement::new(de.sku_id, owner))
}
