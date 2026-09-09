use std::{sync::{Arc, LazyLock}, time::{Duration, SystemTime, UNIX_EPOCH}};
use dashmap::DashMap;

use tracing::instrument;
use twilight_model::{
    application::interaction::{application_command::{CommandData, CommandOptionValue}, modal::ModalInteractionData}, channel::message::{
        Component, MessageFlags,
        component::{TextInput, TextInputStyle},
    }, gateway::payload::incoming::InteractionCreate, id::{
        Id,
        marker::{ChannelMarker, MessageMarker, StickerMarker},
    },
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    message::{FileUploadBuilder, LabelBuilder, TextDisplayBuilder},
};
use uuid::Uuid;

use crate::AppState;

struct ModalInfo {
    channel: Id<ChannelMarker>,
    sticker: Option<Id<StickerMarker>>,
    reply_message_id: Option<Id<MessageMarker>>,
    forward_message_id: Option<Id<MessageMarker>>,
    forward_channel_id: Option<Id<ChannelMarker>>,
    tts: bool,
    mention: bool,
    silent: bool,
    timestamp: u64,
}

static MODAL_INFO_MAP: LazyLock<Arc<DashMap<String, ModalInfo>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

#[instrument(skip_all, err)]
pub async fn run(
    state: AppState,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    let user = event
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("Not a member"))?;
    let user_roles = &user.roles;

    if !user_roles.contains(&Id::new(1410929363863732234)) // co-owner
        || !user_roles.contains(&Id::new(1410929110502608896))
    // owner
    {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("Not allowed to run command")
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        return Ok(()); // just to suppress error
    }

    let mut channel: Id<ChannelMarker> = Id::new(1);
    let mut sticker: Id<StickerMarker> = Id::new(1);
    let mut reply: &str = "";
    let mut forward: &str = "";
    let mut mention_author: bool = false;
    let mut silent: bool = false;
    let mut tts: bool = false;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("channel", CommandOptionValue::String(channel2)) => {
                channel = Id::new(channel2.parse::<u64>()?);
            }
            ("sticker", CommandOptionValue::String(sticker2)) => {
                sticker = Id::new(sticker2.parse::<u64>()?);
            }
            ("reply", CommandOptionValue::String(reply2)) => {
                reply = reply2;
            }
            ("forward", CommandOptionValue::String(forward2)) => {
                forward = forward2;
            }
            ("mention_author", CommandOptionValue::Boolean(mention_author2)) => {
                mention_author = *mention_author2;
            }
            ("silent", CommandOptionValue::Boolean(silent2)) => {
                silent = *silent2;
            }
            ("tts", CommandOptionValue::Boolean(tts2)) => {
                tts = *tts2;
            }
            _ => {}
        }
    }

    let reply_id_str = reply.split("/").nth(7);
    let mut reply_id: Option<Id<MessageMarker>> = None;
    if reply_id_str.is_none() || reply.is_empty() {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("No message id in reply")
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        anyhow::bail!("No message id in reply");
    } else if let Some(id_str) = reply_id_str {
        reply_id = Some(Id::new(id_str.parse::<u64>()?));
    }

    let channel = if !reply.is_empty() {
        let Some(c_id_str) = reply.split('/').nth(5) else {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content("No channel id in reply")
                .flags(MessageFlags::EPHEMERAL)
                .await?;
            anyhow::bail!("No channel id in reply");
        };

        let c_id = match c_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_followup(&event.token)
                    .content(&format!("An error happened:\n{e}"))
                    .flags(MessageFlags::EPHEMERAL)
                    .await?;
                return Err(e.into());
            }
        };

        Id::new(c_id)
    } else if channel.get() == 1 {
        event
            .channel
            .as_ref()
            .ok_or(anyhow::anyhow!("Not called in a channel"))?
            .id
    } else {
        channel
    };

    let forward_info: Option<(Id<ChannelMarker>, Id<MessageMarker>)> = if !forward.is_empty() {
        let mut reply_iter = reply.split('/');
        let Some(c_id_str) = reply_iter.nth(5) else {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content("No channel id in forward link")
                .flags(MessageFlags::EPHEMERAL)
                .await?;
            anyhow::bail!("No channel id in forward");
        };

        let c_id = match c_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_followup(&event.token)
                    .content(&format!("Failed to parse channel id in forward link"))
                    .flags(MessageFlags::EPHEMERAL)
                    .await?;
                return Err(e.into());
            }
        };

        let Some(m_id_str) = reply_iter.next() else {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content("No channel id in forward link")
                .flags(MessageFlags::EPHEMERAL)
                .await?;
            anyhow::bail!("No channel id in forward");
        };

        let m_id = match m_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_followup(&event.token)
                    .content(&format!("Failed to parse message id in forward link"))
                    .flags(MessageFlags::EPHEMERAL)
                    .await?;
                return Err(e.into());
            }
        };

        Some((Id::new(c_id), Id::new(m_id)))
    } else {
        None
    };

    let mut forward_channel = None;
    let mut forward_message = None;
    match forward_info {
        Some((forward_c, forward_m)) => {
            forward_channel = Some(forward_c);
            forward_message = Some(forward_m);
        }
        None => {}
    }

    let modal_id = format!("say_modal:{}", Uuid::new_v4());
    let components = InteractionResponseDataBuilder::new()
        .custom_id(&modal_id)
        .title("Say")
        .components([
            Component::Label(
                LabelBuilder::new(
                    "Content",
                    Component::TextInput(TextInput {
                        id: Some(0),
                        custom_id: "content".to_string(),
                        max_length: Some(2000),
                        min_length: Some(1),
                        required: Some(true),
                        style: TextInputStyle::Paragraph,
                        placeholder: None,
                        #[allow(deprecated)] // can't make a textinput without a deprecated label
                        label: None,
                        value: None,
                    }),
                )
                .description("Type what for bot to say")
                .build(),
            ),
            Component::Label(
                LabelBuilder::new(
                    "File upload",
                    Component::FileUpload(
                        FileUploadBuilder::new("file_upload")
                            .max_values(10)
                            .required(false)
                            .id(1)
                            .build(),
                    ),
                )
                .build(),
            ),
            Component::TextDisplay(
                TextDisplayBuilder::new("# Extra Info:\n## How to make images and videos spoilered?\nDownload a file and add \"SPOILER_\" at the start of a file name, otherwise it won't work")
                .build()
            )
        ])
        .build();

    state
        .client
        .interaction(state.application_id)
        .create_response(
            event.id,
            &event.token,
            &twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::Modal,
                data: Some(components),
            },
        )
        .await?;

    MODAL_INFO_MAP
        .insert(
            modal_id,
            ModalInfo {
                channel,
                sticker: {
                    if sticker.get() == 1 {
                        None
                    } else {
                        Some(sticker)
                    }
                },
                reply_message_id: reply_id,
                forward_message_id: forward_message,
                forward_channel_id: forward_channel,
                tts,
                mention: mention_author,
                silent,
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + Duration::from_hours(1).as_secs(),
            },
        );

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn modal(
    state: AppState,
    event: &Box<InteractionCreate>,
    data: &Box<ModalInteractionData>,
) -> anyhow::Result<()> {
    
    Ok(())
}