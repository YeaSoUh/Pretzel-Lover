use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::CommandData, channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate, id::Id,
};

use crate::{AppState, commands, helpers::sync_configs::update_configs};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    _data: &Box<CommandData>,
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, true).await?;

    if event.author_id().ok_or(anyhow::anyhow!("No author id"))? != Id::new(1021835061433225296) {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("Not allowed to run command")
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        return Ok(()); // just to suppress error
    }

    update_configs(Arc::new(AppState {
        client: state.client.clone(),
        configs: {
            let content = std::fs::read("configs.json")?;
            Arc::new(serde_json::from_slice(&content)?)
        },
        application_id: state.application_id,
    }))?;

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content("Successfully updated configurations!")
        .flags(MessageFlags::EPHEMERAL)
        .await?;

    Ok(())
}
