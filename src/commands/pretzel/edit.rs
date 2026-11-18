use dashmap::DashMap;
use std::{
    fmt::Debug,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::time::Instant;

use tracing::instrument;
use twilight_model::{
    application::interaction::{
        Interaction,
        application_command::{CommandData, CommandOptionValue},
        modal::{ModalInteractionComponent, ModalInteractionData},
    }, channel::message::{
        AllowedMentions, Component, MentionType, MessageFlags,
        component::{TextInput, TextInputStyle},
    }, http::attachment::Attachment, id::{
        Id,
        marker::{ChannelMarker, MessageMarker},
    }, user::User,
};
use twilight_util::builder::{
    interaction_response::{ChannelMessageBuilder, ModalBuilder},
    message::{
        ContainerBuilder, FileUploadBuilder, LabelBuilder, SeparatorBuilder, TextDisplayBuilder,
    },
};
use uuid::Uuid;

use crate::{
    AppState,
    commands::{
        bank::db::internal_types::ComponentV2Logger,
        pretzel::helpers::purge::{Expiring, purge},
    },
};

struct EditResult<'a> {
    user: &'a User,
    message_id: Id<MessageMarker>,
    message_channel_id: Id<ChannelMarker>,
    content: Option<&'a str>,
    json: Option<&'a [u8]>,
    attachments: Option<Vec<Attachment>>,
}

impl ComponentV2Logger for EditResult<'_> {
    fn to_component(&self) -> Component {
        let mut component = ContainerBuilder::new()
            .accent_color(Some(if self.json.is_some() {
                0xFEE75C
            } else {
                0x5865F2
            }))
            .spoiler(false)
            .component(TextDisplayBuilder::new("# EDIT").build())
            .component(SeparatorBuilder::new().build())
            .component(
                TextDisplayBuilder::new(format!(
                    "**User:** {}\n**Channel:** <#{}>\n**Message:** `{}`",
                    self.user.name, self.message_channel_id, self.message_id
                ))
                .build(),
            );

        if let Some(json) = self.json {
            component = component.component(
                TextDisplayBuilder::new(format!(
                    "**JSON**\n```json\n{}\n```",
                    String::from_utf8_lossy(json)
                ))
                .build(),
            );
        } else if let Some(content) = self.content {
            component = component
                .component(SeparatorBuilder::new().build())
                .component(
                    TextDisplayBuilder::new(format!(
                        "**Content**\n> {}",
                        content.replace('\n', "\n> ")
                    ))
                    .build(),
                );
        }

        if let Some(files) = self.attachments.as_ref().filter(|a| !a.is_empty()) {
            let mut list = format!("**Attachments ({})**", files.len());
            for a in files {
                list.push_str(&format!("\n- `{}` ({} B)", a.filename, a.file.len()));
            }
            component = component
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(list).build());
        }

        Component::Container(component.build())
    }
}

impl Debug for EditResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditResult")
            .field("user_id", &self.user.name)
            .field("message_id", &self.message_id)
            .field("message_channel_id", &self.message_channel_id)
            .field("content", &self.content)
            .field("json", &self.json)
            .field(
                "attachments",
                &self.attachments.as_ref().map(|files| {
                    files
                        .iter()
                        .map(|a| (a.filename.as_str(), a.file.len()))
                        .collect::<Vec<_>>()
                }),
            )
            .finish()
    }
}

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

const INAPPROPRIATE_FLAGS: u64 = (1 << 4) | (1 << 6) | (1 << 7);

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: Interaction,
    data: CommandData,
) -> anyhow::Result<()> {
    let user = event
        .member
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not a member"))?;
    let user_roles = &user.roles;

    if (!user_roles.contains(&Id::new(1410929363863732234)) // co-owner
            && !user_roles.contains(&Id::new(1410929110502608896)) // owner
            && !user_roles.contains(&Id::new(1559773167780626453)) // magellanic president
            && !user_roles.contains(&Id::new(1569003594152149063))) // high rank officials
        && user.user.as_ref().ok_or_else(|| anyhow::anyhow!("No user"))?.id != Id::new(1021835061433225296)
    // owner of the bot
    {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &ChannelMessageBuilder::new()
                    .content("Not allowed to run command")
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            )
            .await?;
        return Ok(()); // just to suppress error
    }

    let mut message_channel_id: Id<ChannelMarker> = Id::new(1);
    let mut message_id: Id<MessageMarker> = Id::new(1);

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &data.options {
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
                            &ChannelMessageBuilder::new()
                                .content("No channel id in message url")
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
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
                            &ChannelMessageBuilder::new()
                                .content("No message id in message url")
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
                        )
                        .await?;
                    anyhow::bail!("No message id in message url");
                }
            }
            _ => {}
        }
    }

    let modal_id = format!("edit_modal:{}", Uuid::new_v4());
    let modal = ModalBuilder::new(&modal_id, "Edit", [
        Component::Label(
            LabelBuilder::new(
                "Content",
                Component::TextInput(TextInput {
                    id: Some(0),
                    custom_id: "content".to_string(),
                    max_length: Some(2000),
                    min_length: Some(1),
                    required: Some(false),
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
                "JSON",
                Component::TextInput(TextInput {
                    id: Some(0),
                    custom_id: "JSON".to_string(),
                    max_length: Some(4000),
                    min_length: Some(1),
                    required: Some(false),
                    style: TextInputStyle::Paragraph,
                    placeholder: None,
                    #[allow(deprecated)] // can't make a textinput without a deprecated label
                    label: None,
                    value: None,
                }),
            )
            .description("A JSON field for making custom messages (will overwrite content but not files field)")
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
        .create_response(event.id, &event.token, &modal)
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
    state: Arc<AppState>,
    event: Interaction,
    data: ModalInteractionData,
) -> anyhow::Result<()> {
    let extra_params = MODAL_INFO_MAP
        .remove(&data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Didn't find extra params"))?
        .1;

    let mut text: Option<&str> = None;
    let mut json_bytes: Option<&[u8]> = None;
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
                ModalInteractionComponent::TextInput(val) => match &val.custom_id {
                    str if str == "content" => {
                        text = Some(&val.value);
                    }
                    str if str == "JSON" && !val.value.is_empty() => {
                        json_bytes = Some(val.value.as_bytes());
                    }
                    _ => {}
                },
                ModalInteractionComponent::FileUpload(val) => {
                    for file_id in &val.values {
                        let file = data
                            .resolved
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("No attachments"))?
                            .attachments
                            .get(file_id)
                            .ok_or_else(|| anyhow::anyhow!("Didn't find any attachments"))?;

                        if let Some(flags) = file.flags {
                            if flags.bits() & INAPPROPRIATE_FLAGS != 0 {
                                state
                                    .client
                                    .interaction(state.application_id)
                                    .create_response(
                                        event.id,
                                        &event.token,
                                        &ChannelMessageBuilder::new()
                                            .content("Detected innapropriate content in provided attachments")
                                            .flags(MessageFlags::EPHEMERAL)
                                            .build(),
                                    )
                                    .await?;
                                tracing::error!(
                                    "Detected inappropriate content by {}",
                                    event
                                        .author_id()
                                        .ok_or_else(|| anyhow::anyhow!("No author id"))?
                                        .get()
                                );
                                return Ok(());
                            }
                        }

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

    if text.is_none_or(|val| val.is_empty()) && files.is_empty() && json_bytes.is_none() {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &ChannelMessageBuilder::new()
                    .content("No message content provided to send")
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            )
            .await?;
        anyhow::bail!("No message content provided to send");
    }

    let mut edit_message = state
        .client
        .update_message(extra_params.message_channel_id, extra_params.message_id)
        .content(text)
        .attachments(files.as_slice())
        .allowed_mentions(mentions);
    if let Some(json) = json_bytes {
        edit_message = edit_message.payload_json(json);
    }

    let result = edit_message.await;
    if let Err(e) = result {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &ChannelMessageBuilder::new()
                    .content(&format!(
                        "There was an error while editing a message:\n{}",
                        e.to_string()
                    ))
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            )
            .await?;
        return Err(e.into());
    }

    state
        .client
        .interaction(state.application_id)
        .create_response(
            event.id,
            &event.token,
            &ChannelMessageBuilder::new()
                .content("Message has been edited")
                .flags(MessageFlags::EPHEMERAL)
                .build(),
        )
        .await?;

    let result = EditResult {
        user: event.author().ok_or_else(|| anyhow::anyhow!("No author"))?,
        message_id: extra_params.message_id,
        message_channel_id: extra_params.message_channel_id,
        content: text,
        json: json_bytes,
        attachments: Some(files),
    };

    println!("{:#?}", result);

    if event
        .guild_id
        .ok_or_else(|| anyhow::anyhow!("No guild id"))?
        == Id::new(1559771694145601566)
    {
        state
            .client
            .create_message(state.configs.logging_channel)
            .components(&[result.to_component()])
            .await?;
    }

    Ok(())
}
