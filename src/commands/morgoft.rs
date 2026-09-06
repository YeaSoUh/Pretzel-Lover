use twilight_http::request::AuditLogReason;
use twilight_model::{
    application::interaction::InteractionData, channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate, id::Id,
};

use crate::{AppState, commands};

pub async fn run(state: AppState, event: &Box<InteractionCreate>) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, false).await?;

    let user = event
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("Not a member"))?;
    let user_roles = &user.roles;

    if user_roles.contains(&Id::new(1410929363863732234))
        || user_roles.contains(&Id::new(1410929110502608896))
    {
        // co-owner or owner
        if let Some(InteractionData::ApplicationCommand(data)) = &event.data {
            let target_id = data.target_id.ok_or(anyhow::anyhow!("No target id"))?;

            state
                .client
                .add_guild_member_role(
                    event.guild_id.ok_or(anyhow::anyhow!("No guild id"))?,
                    target_id.cast(),
                    Id::new(1425905354897883290),
                )
                .reason(&format!(
                    "Morgoft'ed by {}",
                    user.nick.as_ref().ok_or(anyhow::anyhow!("No nickname"))?
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
        }
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
