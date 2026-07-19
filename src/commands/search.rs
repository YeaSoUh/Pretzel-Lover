use twilight_model::{
    application::interaction::InteractionData, gateway::payload::incoming::InteractionCreate,
};

use crate::{AppState, commands};

pub async fn run(state: AppState, event: &Box<InteractionCreate>) -> anyhow::Result<()> {
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

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content("TODO command")
        .await?;

    let _options = match event.data.as_ref() {
        Some(InteractionData::ApplicationCommand(data)) => data.options.clone(),
        _ => return Err(anyhow::anyhow!("No options (shouldn't happen)")),
    };

    Ok(())
}
