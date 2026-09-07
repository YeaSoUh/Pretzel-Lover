use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    id::Id,
};

use crate::{
    AppState, commands,
    db::connection::{self, EditRequest},
};

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

    if event
        .author_id()
        .is_none_or(|id| state.configs.users_blacklist.contains(&id))
    {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content("Blacklisted")
            .await?;
        return Ok(());
    }

    let mut index: Option<String> = None;
    let mut input: Option<String> = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("index", CommandOptionValue::String(index2)) => index = Some(index2.clone()),

            ("input", CommandOptionValue::String(input2)) => {
                input = Some(input2.clone());
            }
            _ => {}
        }
    }

    let index = index.ok_or_else(|| anyhow::anyhow!("Missing index option"))?;
    let input = input.ok_or_else(|| anyhow::anyhow!("Missing input option"))?;
    let bypass = &event.author_id().ok_or_else(|| {
        anyhow::anyhow!("shouldn't happen in edit.rs no user id who initiated this command")
    })? == &Id::new(1021835061433225296);

    let result = connection::edit_planet(
        &EditRequest {
            input: input,
            index: index,
        },
        bypass,
    )
    .await;

    if let Err(e) = result {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content(&format!(
                "There was an error while editing a planet:\n{}",
                e.to_string()
            ))
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        return Err(e);
    }

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content("Data was successfully edited!")
        .await?;

    Ok(())
}
