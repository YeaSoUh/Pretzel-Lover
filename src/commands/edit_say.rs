use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::CommandData, channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate, id::Id,
};

use crate::{AppState, commands};

#[instrument(skip_all, err)]
pub async fn run(
    state: AppState,
    event: &Box<InteractionCreate>,
    _data: &Box<CommandData>, // underscored for now
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, false).await?;

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

    Ok(())
}
