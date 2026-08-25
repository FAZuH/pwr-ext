//! Wrapper types for serenity's guild-related request builders: channels,
//! forum posts, threads, invites, scheduled events, stage instances, and
//! webhooks.

use std::borrow::Cow;

use serde::Deserialize;
use serde_json::Value;
use serenity::builder::CreateChannel;
use serenity::builder::CreateForumPost;
use serenity::builder::CreateForumTag;
use serenity::builder::CreateInvite;
use serenity::builder::CreateMessage;
use serenity::builder::CreateScheduledEvent;
use serenity::builder::CreateStageInstance;
use serenity::builder::CreateThread;
use serenity::builder::CreateWebhook;
use serenity::model::channel::AutoArchiveDuration;
use serenity::model::channel::ChannelFlags;
use serenity::model::channel::ChannelType;
use serenity::model::channel::ForumEmoji;
use serenity::model::channel::ForumLayoutType;
use serenity::model::channel::PermissionOverwrite;
use serenity::model::channel::ReactionType;
use serenity::model::channel::SortOrder;
use serenity::model::channel::VideoQualityMode;
use serenity::model::id::EmojiId;
use serenity::model::id::ForumTagId;
use serenity::nonmax::NonMaxU16;

use crate::message::CreateMessageDe;
use crate::util::DataUriDe;
use crate::util::mirror;
use crate::util::opaque_wrapper;

/// Mirror of [`CreateChannel`].
///
/// Lifetime-free like the other newer mirrors. `ChannelType` accepts any
/// number, mirroring upstream's `Unknown(u8)` fallback (spec D5).
#[derive(Debug, Deserialize)]
pub struct CreateChannelDe {
    name: Cow<'static, str>,
    #[serde(rename = "type")]
    kind: ChannelType,
    #[serde(default)]
    topic: Option<Cow<'static, str>>,
    #[serde(default)]
    bitrate: Option<u32>,
    #[serde(default)]
    user_limit: Option<NonMaxU16>,
    #[serde(default)]
    rate_limit_per_user: Option<NonMaxU16>,
    #[serde(default)]
    position: Option<u16>,
    #[serde(default)]
    permission_overwrites: Vec<PermissionOverwrite>,
    #[serde(default)]
    parent_id: Option<serenity::model::id::ChannelId>,
    #[serde(default)]
    nsfw: Option<bool>,
    #[serde(default)]
    rtc_region: Option<Cow<'static, str>>,
    #[serde(default)]
    video_quality_mode: Option<VideoQualityMode>,
    #[serde(default)]
    default_auto_archive_duration: Option<AutoArchiveDuration>,
    #[serde(default)]
    default_reaction_emoji: Option<ForumEmoji>,
    #[serde(default)]
    available_tags: Vec<CreateForumTagDe>,
    #[serde(default)]
    default_sort_order: Option<SortOrder>,
    #[serde(default)]
    default_forum_layout: Option<ForumLayoutType>,
    #[serde(default)]
    default_thread_rate_limit_per_user: Option<NonMaxU16>,
    #[serde(default)]
    flags: Option<ChannelFlags>,
}

impl From<CreateChannelDe> for CreateChannel<'static> {
    fn from(de: CreateChannelDe) -> Self {
        let mut channel = CreateChannel::new(de.name).kind(de.kind);
        if let Some(topic) = de.topic {
            channel = channel.topic(topic);
        }
        if let Some(bitrate) = de.bitrate {
            channel = channel.bitrate(bitrate);
        }
        if let Some(user_limit) = de.user_limit {
            channel = channel.user_limit(user_limit);
        }
        if let Some(rate_limit_per_user) = de.rate_limit_per_user {
            channel = channel.rate_limit_per_user(rate_limit_per_user);
        }
        if let Some(position) = de.position {
            channel = channel.position(position);
        }
        if !de.permission_overwrites.is_empty() {
            channel = channel.permissions(de.permission_overwrites);
        }
        if let Some(parent_id) = de.parent_id {
            channel = channel.category(parent_id);
        }
        if let Some(nsfw) = de.nsfw {
            channel = channel.nsfw(nsfw);
        }
        if let Some(rtc_region) = de.rtc_region {
            channel = channel.rtc_region(rtc_region);
        }
        if let Some(video_quality_mode) = de.video_quality_mode {
            channel = channel.video_quality_mode(video_quality_mode);
        }
        if let Some(duration) = de.default_auto_archive_duration {
            channel = channel.default_auto_archive_duration(duration);
        }
        if let Some(emoji) = de.default_reaction_emoji {
            channel = channel.default_reaction_emoji(emoji);
        }
        if !de.available_tags.is_empty() {
            channel = channel.available_tags(
                de.available_tags
                    .into_iter()
                    .map(CreateForumTag::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(sort_order) = de.default_sort_order {
            channel = channel.default_sort_order(sort_order);
        }
        if let Some(forum_layout) = de.default_forum_layout {
            channel = channel.default_forum_layout(forum_layout);
        }
        if let Some(rate_limit) = de.default_thread_rate_limit_per_user {
            channel = channel.default_thread_rate_limit_per_user(rate_limit);
        }
        if let Some(flags) = de.flags {
            channel = channel.flags(flags);
        }
        channel
    }
}

/// Mirror of [`CreateForumTag`].
///
/// All four fields serialize unconditionally upstream — unset emojis
/// rebuild as explicit `null`s. Carrying both emoji fields at once is
/// rejected, mirroring what the builder can represent.
#[derive(Debug)]
pub struct CreateForumTagDe(CreateForumTag<'static>);

opaque_wrapper!(CreateForumTagDe, CreateForumTag<'static>, parse_forum_tag);

#[derive(Debug, Deserialize)]
struct RawForumTagDe {
    name: Cow<'static, str>,
    moderated: bool,
    #[serde(default)]
    emoji_id: Option<EmojiId>,
    #[serde(default)]
    emoji_name: Option<Cow<'static, str>>,
}

fn parse_forum_tag(value: &Value) -> Result<CreateForumTag<'static>, String> {
    let de: RawForumTagDe = mirror(value)?;
    let mut tag = CreateForumTag::new(de.name).moderated(de.moderated);
    // The setter splits reactions into id/name; a custom reaction built
    // from an `EmojiId` carries a placeholder name that the setter
    // discards, so only the id survives.
    match (de.emoji_id, de.emoji_name) {
        (None, None) => {}
        (Some(emoji_id), None) => tag = tag.emoji(emoji_id),
        (None, Some(emoji_name)) => {
            // Upstream's own string conversion truncates over-long names
            // into the fixed-size emoji buffer; malformed custom-emoji
            // syntax (`<a:name:id>`) is rejected here.
            let reaction = ReactionType::try_from(emoji_name.into_owned())
                .map_err(|_| "invalid unicode emoji name".to_string())?;
            tag = tag.emoji(reaction);
        }
        (Some(_), Some(_)) => {
            return Err("a forum tag cannot carry both emoji_id and emoji_name".to_string());
        }
    }
    Ok(tag)
}

/// Mirror of [`CreateForumPost`]; nests the full [`CreateMessageDe`] mirror.
#[derive(Debug, Deserialize)]
pub struct CreateForumPostDe {
    name: Cow<'static, str>,
    #[serde(default)]
    auto_archive_duration: Option<AutoArchiveDuration>,
    #[serde(default)]
    rate_limit_per_user: Option<NonMaxU16>,
    message: CreateMessageDe,
    #[serde(default)]
    applied_tags: Vec<ForumTagId>,
}

impl From<CreateForumPostDe> for CreateForumPost<'static> {
    fn from(de: CreateForumPostDe) -> Self {
        let mut post = CreateForumPost::new(de.name, CreateMessage::from(de.message));
        if let Some(duration) = de.auto_archive_duration {
            post = post.auto_archive_duration(duration);
        }
        if let Some(rate_limit_per_user) = de.rate_limit_per_user {
            post = post.rate_limit_per_user(rate_limit_per_user);
        }
        if !de.applied_tags.is_empty() {
            post = post.set_applied_tags(de.applied_tags);
        }
        post
    }
}

/// Mirror of [`CreateThread`]. `type` accepts any number, mirroring
/// upstream's `Unknown(u8)` fallback on [`ChannelType`].
#[derive(Debug, Deserialize)]
pub struct CreateThreadDe {
    name: Cow<'static, str>,
    #[serde(default)]
    auto_archive_duration: Option<AutoArchiveDuration>,
    #[serde(rename = "type", default)]
    kind: Option<ChannelType>,
    #[serde(default)]
    invitable: Option<bool>,
    #[serde(default)]
    rate_limit_per_user: Option<NonMaxU16>,
}

impl From<CreateThreadDe> for CreateThread<'static> {
    fn from(de: CreateThreadDe) -> Self {
        let mut thread = CreateThread::new(de.name);
        if let Some(duration) = de.auto_archive_duration {
            thread = thread.auto_archive_duration(duration);
        }
        if let Some(kind) = de.kind {
            thread = thread.kind(kind);
        }
        if let Some(invitable) = de.invitable {
            thread = thread.invitable(invitable);
        }
        if let Some(rate_limit_per_user) = de.rate_limit_per_user {
            thread = thread.rate_limit_per_user(rate_limit_per_user);
        }
        thread
    }
}

/// Mirror of [`CreateInvite`]; every field is optional both on the wire and
/// in the builder.
#[derive(Debug, Deserialize)]
pub struct CreateInviteDe {
    #[serde(default)]
    max_age: Option<u32>,
    #[serde(default)]
    max_uses: Option<u8>,
    #[serde(default)]
    temporary: Option<bool>,
    #[serde(default)]
    unique: Option<bool>,
    #[serde(default)]
    target_type: Option<serenity::model::invite::InviteTargetType>,
    #[serde(default)]
    target_user_id: Option<serenity::model::id::UserId>,
    #[serde(default)]
    target_application_id: Option<serenity::model::id::ApplicationId>,
    #[serde(default)]
    role_ids: Option<Vec<serenity::model::id::RoleId>>,
}

impl From<CreateInviteDe> for CreateInvite<'static> {
    fn from(de: CreateInviteDe) -> Self {
        let mut invite = CreateInvite::new();
        if let Some(max_age) = de.max_age {
            invite = invite.max_age(max_age);
        }
        if let Some(max_uses) = de.max_uses {
            invite = invite.max_uses(max_uses);
        }
        if let Some(temporary) = de.temporary {
            invite = invite.temporary(temporary);
        }
        if let Some(unique) = de.unique {
            invite = invite.unique(unique);
        }
        if let Some(target_type) = de.target_type {
            invite = invite.target_type(target_type);
        }
        if let Some(target_user_id) = de.target_user_id {
            invite = invite.target_user_id(target_user_id);
        }
        if let Some(target_application_id) = de.target_application_id {
            invite = invite.target_application_id(target_application_id);
        }
        if let Some(role_ids) = de.role_ids {
            invite = invite.role_ids(role_ids);
        }
        invite
    }
}

/// Mirror of [`CreateScheduledEvent`], including the private metadata
/// mirror (`{"location": ... | null}`).
///
/// Upstream pins `privacy_level` to `GuildOnly` at construction and offers
/// no setter, so any other wire value normalizes to `2`.
#[derive(Debug, Deserialize)]
pub struct CreateScheduledEventDe {
    #[serde(default)]
    channel_id: Option<serenity::model::id::ChannelId>,
    #[serde(default)]
    entity_metadata: Option<ScheduledEventMetadataDe>,
    name: Cow<'static, str>,
    privacy_level: serenity::model::guild::ScheduledEventPrivacyLevel,
    scheduled_start_time: serenity::model::Timestamp,
    #[serde(default)]
    scheduled_end_time: Option<serenity::model::Timestamp>,
    #[serde(default)]
    description: Option<Cow<'static, str>>,
    entity_type: serenity::model::guild::ScheduledEventType,
    #[serde(default)]
    image: Option<DataUriDe>,
}

#[derive(Debug, Deserialize)]
struct ScheduledEventMetadataDe {
    location: Option<Cow<'static, str>>,
}

impl From<CreateScheduledEventDe> for CreateScheduledEvent<'static> {
    fn from(de: CreateScheduledEventDe) -> Self {
        let _ = de.privacy_level; // always rebuilt as GuildOnly by new()
        let mut event =
            CreateScheduledEvent::new(de.entity_type, de.name.clone(), de.scheduled_start_time);
        if let Some(channel_id) = de.channel_id {
            event = event.channel_id(channel_id);
        }
        // A metadata object without a location cannot be expressed through
        // the public setters and normalizes to no metadata at all.
        if let Some(Some(location)) = de.entity_metadata.map(|metadata| metadata.location) {
            event = event.location(location);
        }
        if let Some(end_time) = de.scheduled_end_time {
            event = event.end_time(end_time);
        }
        if let Some(description) = de.description {
            event = event.description(description);
        }
        if let Some(image) = de.image {
            event = event.image(image.into_data_uri());
        }
        event
    }
}

/// Mirror of [`CreateStageInstance`].
///
/// Upstream fills `channel_id` only inside its HTTP executor and offers no
/// public setter, so the accepted-but-required-on-the-wire value is dropped
/// on rebuild (always `null`). `privacy_level` has no public setter either
/// and always rebuilds as `GuildOnly` (`2`).
#[derive(Debug, Deserialize)]
pub struct CreateStageInstanceDe {
    #[allow(dead_code)]
    channel_id: Option<serenity::model::id::ChannelId>,
    topic: Cow<'static, str>,
    privacy_level: serenity::model::channel::StageInstancePrivacyLevel,
    #[serde(default)]
    send_start_notification: Option<bool>,
}

impl From<CreateStageInstanceDe> for CreateStageInstance<'static> {
    fn from(de: CreateStageInstanceDe) -> Self {
        let _ = (&de.channel_id, &de.privacy_level); // not settable publicly
        let mut stage = CreateStageInstance::new(de.topic);
        if let Some(send_start_notification) = de.send_start_notification {
            stage = stage.send_start_notification(send_start_notification);
        }
        stage
    }
}

/// Mirror of [`CreateWebhook`].
#[derive(Debug, Deserialize)]
pub struct CreateWebhookDe {
    name: Cow<'static, str>,
    #[serde(default)]
    avatar: Option<DataUriDe>,
}

impl From<CreateWebhookDe> for CreateWebhook<'static> {
    fn from(de: CreateWebhookDe) -> Self {
        let webhook = CreateWebhook::new(de.name);
        match de.avatar {
            Some(avatar) => webhook.avatar(avatar.into_data_uri()),
            None => webhook,
        }
    }
}
