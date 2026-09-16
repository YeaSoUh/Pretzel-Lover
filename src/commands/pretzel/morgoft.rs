use tracing::instrument;
use twilight_http::request::AuditLogReason;
use twilight_model::{
    application::interaction::application_command::CommandData, channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate, id::Id,
};

use crate::{AppState, commands};

#[instrument(skip_all, err)]
pub async fn run(
    state: AppState,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, false).await?;

    let member = event
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("Not a member"))?;
    let member_roles = &member.roles;

    if member_roles.contains(&Id::new(1410929363863732234)) // co-owner
        || member_roles.contains(&Id::new(1410929110502608896))
    // owner
    {
        let target_id = data.target_id.ok_or(anyhow::anyhow!("No target id"))?;
        let user = member
            .user
            .as_ref()
            .ok_or(anyhow::anyhow!("No user (as PartialMember)"))?;

        state
            .client
            .add_guild_member_role(
                event.guild_id.ok_or(anyhow::anyhow!("No guild id"))?,
                target_id.cast(),
                Id::new(1425905354897883290),
            )
            .reason(&format!(
                "Morgoft'ed by {}",
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
            .content(&format!(
                "<@{}> HAS BEEN SENT TO THE WORST PLACE",
                target_id
            ))
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
