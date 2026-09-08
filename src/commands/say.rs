use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::{
        Component, MessageFlags,
        component::{TextInput, TextInputStyle},
    },
    gateway::payload::incoming::InteractionCreate,
    id::{
        Id,
        marker::{ChannelMarker, StickerMarker},
    },
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    message::{FileUploadBuilder, LabelBuilder},
};
use uuid::Uuid;

use crate::AppState;

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

    let mut channel: Id<ChannelMarker> = Id::new(0);
    let mut sticker: Id<StickerMarker> = Id::new(0);
    let mut reply: &str = "";
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
    } else if channel.get() == 0 {
        event
            .channel
            .as_ref()
            .ok_or(anyhow::anyhow!("Not called in a channel"))?
            .id
    } else {
        channel
    };

    let components = InteractionResponseDataBuilder::new()
        .custom_id(format!("say_modal:{}", Uuid::new_v4()))
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

    Ok(())
}
