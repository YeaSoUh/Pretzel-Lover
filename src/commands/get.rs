use twilight_model::{
    application::interaction::{InteractionData, application_command::CommandOptionValue},
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{AppState, commands, db::connection::{self, PlanetQuery}};

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
    let mut index: Option<String> = None;

    if let Some(InteractionData::ApplicationCommand(cmd_box)) = &event.data {
        let cmd = cmd_box.as_ref();

        for option in &cmd.options {
            match (&*option.name, &option.value) {
                ("index", CommandOptionValue::String(index2)) => {
                    index = Some(index2.clone());
                }
                _ => {}
            }
        }
    } else {
        anyhow::bail!("No options");
    }

    let id = index.ok_or_else(|| anyhow::anyhow!("Missing id option"))?;

    let planet = match connection::get_planet(
        &PlanetQuery {
            input: None,
            index: Some(id),
            input_style: None,
        },
        state.clone(),
    ).await {
        Ok(planet) => planet,
        Err(_) => {
            state
                .client
                .interaction(state.application_id)
                .create_followup(&event.token)
                .content("Planet doesn't exist in database or an error happened internally that was reported")
                .flags(MessageFlags::EPHEMERAL)
                .await?;
            return Ok(());
        }
    };

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content(&planet.format(true))
        .await?;

    Ok(())
}
