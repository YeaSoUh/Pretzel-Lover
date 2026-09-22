use std::sync::Arc;

use tracing::instrument;
use twilight_http::request::AuditLogReason;
use twilight_model::{
    application::interaction::application_command::CommandData, channel::message::MessageFlags, gateway::payload::incoming::InteractionCreate, id::{Id, marker::RoleMarker},
};

use crate::{AppState, commands};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, true).await?;

    let member = event
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("Not a member"))?;
    let user = member
        .user
        .as_ref()
        .ok_or(anyhow::anyhow!("No user (as PartialMember)"))?;

    let company_role_id: Option<Id<RoleMarker>> = if user.id == Id::new(1317504235495227392) {
        Some(Id::new(1530896915275841607))
    } else if user.id == Id::new(1347832381716828213) {
        Some(Id::new(1532399786949345360))
    } else {
        None
    };

    if let Some(role_id) = company_role_id {
        let target_id = data.target_id.ok_or(anyhow::anyhow!("No target id"))?;

        state
            .client
            .add_guild_member_role(
                event.guild_id.ok_or(anyhow::anyhow!("No guild id"))?,
                target_id.cast(),
                role_id,
            )
            .reason(&format!(
                "Promoted by {}",
                user.global_name
                    .as_deref()
                    .map(String::from)
                    .unwrap_or_else(|| format!("{}#{}", user.name, user.discriminator))
            ))
            .await?;
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("Target has been given an employee role")
            .await?;
    } else {
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
