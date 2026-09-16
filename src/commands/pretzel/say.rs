use dashmap::DashMap;
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::time::Instant;

use tracing::instrument;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandOptionValue},
        modal::{ModalInteractionComponent, ModalInteractionData},
    },
    channel::message::{
        AllowedMentions, Component, MentionType, MessageFlags,
        component::{TextInput, TextInputStyle},
    },
    gateway::payload::incoming::InteractionCreate,
    http::attachment::Attachment,
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, StickerMarker},
    },
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    message::{FileUploadBuilder, LabelBuilder, TextDisplayBuilder},
};
use uuid::Uuid;

use crate::{
    AppState,
    commands::{
        pretzel::helpers::purge::{Expiring, purge},
        response,
    },
};

struct ModalInfo {
    channel: Id<ChannelMarker>,
    sticker: Option<Id<StickerMarker>>,
    reply_message_id: Option<Id<MessageMarker>>,
    forward_message_id: Option<Id<MessageMarker>>,
    forward_channel_id: Option<Id<ChannelMarker>>,
    tts: bool,
    mention: bool,
    silent: bool,
    expires_at: Instant,
}

impl Expiring for ModalInfo {
    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

static MODAL_INFO_MAP: LazyLock<Arc<DashMap<String, Arc<ModalInfo>>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub fn run_once() {
    tokio::spawn(purge(Arc::clone(&MODAL_INFO_MAP), Duration::from_hours(1)));
}

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
        && !user_roles.contains(&Id::new(1410929110502608896))
    // owner
    {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &response(
                    InteractionResponseDataBuilder::new()
                        .content("Not allowed to run command")
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        return Ok(()); // just to suppress error
    }

    let mut channel: Id<ChannelMarker> = event
        .channel
        .as_ref()
        .ok_or(anyhow::anyhow!("Not called in a channel"))?
        .id;
    let mut sticker: Id<StickerMarker> = Id::new(1);
    let mut reply: &str = "";
    let mut forward: &str = "";
    let mut mention_author: bool = false;
    let mut silent: bool = false;
    let mut tts: bool = false;

    let cmd = data.as_ref();

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("channel", CommandOptionValue::Channel(channel2)) => {
                channel = *channel2;
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

    let reply_id_str = reply.split("/").nth(6);
    let mut reply_id: Option<Id<MessageMarker>> = None;
    if reply_id_str.is_none() && !reply.is_empty() {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &response(
                    InteractionResponseDataBuilder::new()
                        .content("No message id in reply")
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        anyhow::bail!("No message id in reply");
    } else if let Some(id_str) = reply_id_str {
        reply_id = Some(Id::new(id_str.parse::<u64>()?));
    }

    if !reply.is_empty() {
        let Some(c_id_str) = reply.split('/').nth(5) else {
            state
                .client
                .interaction(state.application_id)
                .create_response(
                    event.id,
                    &event.token,
                    &response(
                        InteractionResponseDataBuilder::new()
                            .content("No channel id in reply")
                            .flags(MessageFlags::EPHEMERAL)
                            .build(),
                    ),
                )
                .await?;
            anyhow::bail!("No channel id in reply");
        };

        let c_id = match c_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_response(
                        event.id,
                        &event.token,
                        &response(
                            InteractionResponseDataBuilder::new()
                                .content(&format!("An error happened:\n{e}"))
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
                        ),
                    )
                    .await?;
                return Err(e.into());
            }
        };

        channel = Id::new(c_id);
    }

    let forward_info: Option<(Id<ChannelMarker>, Id<MessageMarker>)> = if !forward.is_empty() {
        let mut forward_iter = forward.split('/');
        let Some(c_id_str) = forward_iter.nth(5) else {
            state
                .client
                .interaction(state.application_id)
                .create_response(
                    event.id,
                    &event.token,
                    &response(
                        InteractionResponseDataBuilder::new()
                            .content("Failed to parse channel id in forward link")
                            .flags(MessageFlags::EPHEMERAL)
                            .build(),
                    ),
                )
                .await?;
            anyhow::bail!("No channel id in forward");
        };

        let c_id = match c_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_response(
                        event.id,
                        &event.token,
                        &response(
                            InteractionResponseDataBuilder::new()
                                .content("Failed to parse channel id in forward link")
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
                        ),
                    )
                    .await?;
                return Err(e.into());
            }
        };

        let Some(m_id_str) = forward_iter.next() else {
            state
                .client
                .interaction(state.application_id)
                .create_response(
                    event.id,
                    &event.token,
                    &response(
                        InteractionResponseDataBuilder::new()
                            .content("No channel id in forward link")
                            .flags(MessageFlags::EPHEMERAL)
                            .build(),
                    ),
                )
                .await?;
            anyhow::bail!("No channel id in forward");
        };

        let m_id = match m_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                state
                    .client
                    .interaction(state.application_id)
                    .create_response(
                        event.id,
                        &event.token,
                        &response(
                            InteractionResponseDataBuilder::new()
                                .content("Failed to parse message id in forward link")
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
                        ),
                    )
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

    MODAL_INFO_MAP.insert(
        modal_id,
        Arc::new(ModalInfo {
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
            expires_at: Instant::now() + Duration::from_hours(1),
        }),
    );

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn modal(
    state: AppState,
    event: &Box<InteractionCreate>,
    data: &Box<ModalInteractionData>,
) -> anyhow::Result<()> {
    let extra_params = MODAL_INFO_MAP
        .remove(&data.custom_id)
        .ok_or(anyhow::anyhow!("Didn't find extra params"))?
        .1;

    let mut text: &str = "";
    let mut files: Vec<Attachment> = Vec::new();
    let mentions = Some(&AllowedMentions {
        parse: vec![
            MentionType::Everyone,
            MentionType::Users,
            MentionType::Roles,
        ],
        replied_user: extra_params.mention,
        ..Default::default()
    });
    let sticker_ids = extra_params
        .sticker
        .as_ref()
        .map(|sticker_id| vec![sticker_id.clone()]);

    for component in &data.components {
        match component {
            ModalInteractionComponent::Label(sub_comp) => match &*sub_comp.component {
                ModalInteractionComponent::TextInput(val) => {
                    text = &val.value;
                }
                ModalInteractionComponent::FileUpload(val) => {
                    for file_id in &val.values {
                        let file = data
                            .resolved
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("No attachments"))?
                            .attachments
                            .get(file_id)
                            .ok_or_else(|| anyhow::anyhow!("Didn't find any attachments"))?;
                        files.push(Attachment::from_bytes(
                            file.filename.clone(),
                            reqwest::get(&file.proxy_url).await?.bytes().await?.to_vec(),
                            file_id.get(),
                        ));
                    }
                }
                _ => continue,
            },
            _ => continue,
        }
    }

    if text.is_empty() && files.is_empty() && extra_params.sticker.is_none() {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &response(
                    InteractionResponseDataBuilder::new()
                        .content("No message content provided to send")
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        anyhow::bail!("No message content provided to send");
    }

    let mut create_message = state
        .client
        .create_message(extra_params.channel)
        .content(text)
        .attachments(files.as_slice())
        .allowed_mentions(mentions)
        .flags(if extra_params.silent {
            MessageFlags::SUPPRESS_NOTIFICATIONS
        } else {
            MessageFlags::empty()
        })
        .tts(extra_params.tts);
    if let Some(reply_message_id) = extra_params.reply_message_id {
        create_message = create_message.reply(reply_message_id);
    };
    if let Some(sticker_ids) = sticker_ids.as_ref() {
        create_message = create_message.sticker_ids(sticker_ids.as_slice());
    }

    let result = create_message.await;
    if let Err(e) = result {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &response(
                    InteractionResponseDataBuilder::new()
                        .content(&format!(
                            "There was an error while sending a message:\n{}",
                            e.to_string()
                        ))
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        return Err(e.into());
    }

    if let Some(forward_channel_id) = extra_params.forward_channel_id {
        if let Err(e) = state
            .client
            .create_message(extra_params.channel)
            .forward(
                forward_channel_id,
                extra_params
                    .forward_message_id
                    .ok_or(anyhow::anyhow!("Didn't find forward message id"))?,
            )
            .await
        {
            state
                .client
                .interaction(state.application_id)
                .create_response(
                    event.id,
                    &event.token,
                    &response(
                        InteractionResponseDataBuilder::new()
                            .content(&format!(
                                "There was an error while sending a message:\n{}",
                                e.to_string()
                            ))
                            .flags(MessageFlags::EPHEMERAL)
                            .build(),
                    ),
                )
                .await?;
            return Err(e.into());
        }
    }

    state
        .client
        .interaction(state.application_id)
        .create_response(
            event.id,
            &event.token,
            &response(
                InteractionResponseDataBuilder::new()
                    .content("Message has been sent")
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            ),
        )
        .await?;

    Ok(())
}
