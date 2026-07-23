use twilight_model::{
    application::interaction::{InteractionData, application_command::CommandOptionValue}, gateway::payload::incoming::InteractionCreate,
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

    let mut input: Option<String> = None;

    if let Some(InteractionData::ApplicationCommand(cmd_box)) = &event.data {
        let cmd = cmd_box.as_ref();

        for option in &cmd.options {
            match (&*option.name, &option.value) {
                ("input", CommandOptionValue::String(input2)) => {
                    input = Some(input2.clone());
                }
                _ => {}
            }
        }
    } else {
        anyhow::bail!("No options");
    }

    let attachment = connection::search_planets(&mut input.unwrap(), state.clone()).await?;

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .attachments(&[attachment])
        .await?;

    Ok(())
}
