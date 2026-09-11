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
        marker::{ChannelMarker, MessageMarker},
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
    message_id: Id<MessageMarker>,
    message_channel_id: Id<ChannelMarker>,
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

    let mut message_channel_id: Id<ChannelMarker> = Id::new(1);
    let mut message_id: Id<MessageMarker> = Id::new(1);

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("message_url", CommandOptionValue::String(message_url)) => {
                let mut url_iter = message_url.split("/");

                if let Some(channel_id) = url_iter.nth(5) {
                    message_channel_id = Id::new(channel_id.parse::<u64>()?);
                } else {
                    state
                        .client
                        .interaction(state.application_id)
                        .create_response(
                            event.id,
                            &event.token,
                            &response(
                                InteractionResponseDataBuilder::new()
                                    .content("No channel id in message url")
                                    .flags(MessageFlags::EPHEMERAL)
                                    .build(),
                            ),
                        )
                        .await?;
                    anyhow::bail!("No channel id in message url");
                }

                if let Some(msg_id) = url_iter.next() {
                    message_id = Id::new(msg_id.parse::<u64>()?);
                } else {
                    state
                        .client
                        .interaction(state.application_id)
                        .create_response(
                            event.id,
                            &event.token,
                            &response(
                                InteractionResponseDataBuilder::new()
                                    .content("No message id in message url")
                                    .flags(MessageFlags::EPHEMERAL)
                                    .build(),
                            ),
                        )
                        .await?;
                    anyhow::bail!("No message id in message url");
                }
            }
            _ => {}
        }
    }

    let modal_id = format!("edit_modal:{}", Uuid::new_v4());
    let components = InteractionResponseDataBuilder::new()
        .custom_id(&modal_id)
        .title("Edit")
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
            message_id: message_id,
            message_channel_id: message_channel_id,
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

    let mut text: Option<&str> = None;
    let mut files: Vec<Attachment> = Vec::new();
    let mentions = Some(&AllowedMentions {
        parse: vec![
            MentionType::Everyone,
            MentionType::Users,
            MentionType::Roles,
        ],
        replied_user: true,
        ..Default::default()
    });

    for component in &data.components {
        match component {
            ModalInteractionComponent::Label(sub_comp) => match &*sub_comp.component {
                ModalInteractionComponent::TextInput(val) => {
                    text = Some(&val.value);
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

    if text.is_none_or(|val| val.is_empty()) && files.is_empty() {
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

    let edit_message = state
        .client
        .update_message(extra_params.message_channel_id, extra_params.message_id)
        .content(text)
        .attachments(files.as_slice())
        .allowed_mentions(mentions);

    let result = edit_message.await;
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
                            "There was an error while editing a message:\n{}",
                            e.to_string()
                        ))
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        return Err(e.into());
    }

    Ok(())
}
