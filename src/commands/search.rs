use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{AppState, commands, db::connection};

#[instrument(skip_all, err)]
pub async fn run(
    state: AppState,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, false).await?;
    if event
        .channel
        .as_ref()
        .is_none_or(|chn| !state.configs.allowed_channels.contains(&chn.id))
    {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("Outside of allowed channels")
            .await?;
        return Ok(());
    }

    let mut input: Option<String> = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("input", CommandOptionValue::String(input2)) => {
                input = Some(input2.clone());
            }
            _ => {}
        }
    }

    let input = input.ok_or_else(|| anyhow::anyhow!("No input"))?;

    let result = connection::search_planets(&input, state.clone()).await;

    match result {
        Ok(attachment) => {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .attachments(&[attachment])
                .await?;
        }
        Err(e) => {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content(&format!("An error happened:\n{}", e.to_string()))
                .flags(MessageFlags::EPHEMERAL)
                .await?;
            return Err(e);
        }
    }

    Ok(())
}
