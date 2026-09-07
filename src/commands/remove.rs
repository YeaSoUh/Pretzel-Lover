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
    let mut index: Option<String> = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("index", CommandOptionValue::String(index2)) => {
                index = Some(index2.clone());
            }
            _ => {}
        }
    }

    let id = index.ok_or_else(|| anyhow::anyhow!("Missing id option"))?;

    let result = connection::remove_planet(&id, state.clone()).await;

    if let Err(e) = result {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content(&format!("An error happened:\n{}", e.to_string()))
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        return Err(e);
    }

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content("Data was successfully removed!")
        .await?;

    Ok(())
}
