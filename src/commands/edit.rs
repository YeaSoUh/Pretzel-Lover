// if event.channel.as_ref().is_none_or(|chn| !state.configs._allowed_channels.contains(&chn.id)) { return }

use anyhow::Ok;
use twilight_model::{
    application::interaction::{InteractionData, application_command::CommandOptionValue},
    gateway::payload::incoming::InteractionCreate,
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
    let mut index: Option<String> = None;
    let mut input: Option<String> = None;

    if let Some(InteractionData::ApplicationCommand(cmd_box)) = &event.data {
        let cmd = cmd_box.as_ref();

        for option in &cmd.options {
            match (&*option.name, &option.value) {
                ("index", CommandOptionValue::String(index2)) => {
                    index = Some(index2.clone());
                }
                ("input", CommandOptionValue::String(input2)) => {
                    input = Some(input2.clone());
                }
                _ => {}
            }
        }
    } else {
        return Err(anyhow::anyhow!("No options"));
    }

    let _index = index.ok_or_else(|| anyhow::anyhow!("Missing index option"))?;
    let _input = input.ok_or_else(|| anyhow::anyhow!("Missing input option"))?;

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

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content("TODO command")
        .await?;

    Ok(())
}
