use twilight_model::{
    application::interaction::{InteractionData, application_command::CommandOptionValue},
    gateway::payload::incoming::InteractionCreate,
};

use crate::{AppState, commands, db::connection};

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
    let mut id: Option<String> = None;

    if let Some(InteractionData::ApplicationCommand(cmd_box)) = &event.data {
        let cmd = cmd_box.as_ref();

        for option in &cmd.options {
            match (&*option.name, &option.value) {
                ("id", CommandOptionValue::String(id2)) => {
                    id = Some(id2.clone());
                }
                _ => {}
            }
        }
    } else {
        return Err(anyhow::anyhow!("No options"));
    }

    let id = id.ok_or_else(|| anyhow::anyhow!("Missing id option"))?;

    let planet = match connection::get_planet(&id).await? {
        Ok(planet) => planet,
        Err(_) => {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content("Planet doesn't exist in database")
                .await?;
            return Ok(());
        }
    };

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content(&connection::format_response(planet))
        .await?;

    Ok(())
}
