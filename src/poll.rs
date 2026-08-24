//! Wrapper types for serenity's poll and soundboard builders.

use std::borrow::Cow;
use std::time::Duration;

use serde::Deserialize;
use serenity::builder::CreatePoll;
use serenity::builder::CreatePollAnswer;
use serenity::builder::CreateSoundboard;
use serenity::model::channel::PollLayoutType;
use serenity::model::channel::PollMediaEmoji;

use crate::util::DataUriDe;

/// Mirror of [`CreatePoll`], pinned to the `Ready` typestate stage (spec
/// Design: the phantom-marker stages collapse during deserialization).
///
/// Upstream serializes every field unconditionally, including
/// `layout_type` as an explicit `null` when unset, so plain `Option`s
/// reproduce the wire shape exactly. Any `layout_type` number is accepted,
/// mirroring upstream's `Unknown(u8)` fallback.
#[derive(Debug, Deserialize)]
pub struct CreatePollDe {
    question: PollQuestionDe,
    #[serde(default)]
    answers: Vec<CreatePollAnswerDe>,
    duration: u16,
    allow_multiselect: bool,
    layout_type: Option<PollLayoutType>,
}

#[derive(Debug, Deserialize)]
struct PollQuestionDe {
    text: Cow<'static, str>,
}

impl From<CreatePollDe> for CreatePoll<'static, serenity::builder::create_poll::Ready> {
    fn from(de: CreatePollDe) -> Self {
        let mut poll = CreatePoll::new()
            .question(de.question.text)
            .answers(
                de.answers
                    .into_iter()
                    .map(CreatePollAnswer::from)
                    .collect::<Vec<_>>(),
            )
            .duration(Duration::from_secs(u64::from(de.duration) * 3600));
        if de.allow_multiselect {
            poll = poll.allow_multiselect();
        }
        if let Some(layout_type) = de.layout_type {
            poll = poll.layout_type(layout_type);
        }
        poll
    }
}

/// Mirror of [`CreatePollAnswer`].
#[derive(Debug, Deserialize)]
pub struct CreatePollAnswerDe {
    poll_media: PollAnswerMediaDe,
}

// Upstream has no skip attributes here either: unset text/emoji rebuild as
// explicit `null`s, matching its wire shape.
#[derive(Debug, Deserialize)]
struct PollAnswerMediaDe {
    #[serde(default)]
    text: Option<Cow<'static, str>>,
    #[serde(default)]
    emoji: Option<PollMediaEmoji>,
}

impl From<CreatePollAnswerDe> for CreatePollAnswer<'static> {
    fn from(de: CreatePollAnswerDe) -> Self {
        let mut answer = CreatePollAnswer::new();
        if let Some(text) = de.poll_media.text {
            answer = answer.text(text);
        }
        if let Some(emoji) = de.poll_media.emoji {
            answer = answer.emoji(emoji);
        }
        answer
    }
}

/// Mirror of [`CreateSoundboard`].
///
/// Lifetime-free: the sound is validated while deserializing, which
/// materializes an owned string regardless.
#[derive(Debug, Deserialize)]
pub struct CreateSoundboardDe {
    name: Cow<'static, str>,
    sound: DataUriDe,
    volume: f64,
    #[serde(default)]
    emoji_id: Option<serenity::model::id::EmojiId>,
    #[serde(default)]
    emoji_name: Option<Cow<'static, str>>,
}

impl From<CreateSoundboardDe> for CreateSoundboard<'static> {
    fn from(de: CreateSoundboardDe) -> Self {
        // The URI shape was validated at deserialization time; upstream's
        // constructor re-checks it and cannot fail here.
        let sound = serenity::builder::DataUri::from_base64(de.sound.0)
            .expect("data URI validated during deserialization");
        let mut soundboard = CreateSoundboard::new(de.name, sound).volume(de.volume);
        if let Some(emoji_id) = de.emoji_id {
            soundboard = soundboard.emoji_id(emoji_id);
        }
        if let Some(emoji_name) = de.emoji_name {
            soundboard = soundboard.emoji_name(emoji_name);
        }
        soundboard
    }
}
