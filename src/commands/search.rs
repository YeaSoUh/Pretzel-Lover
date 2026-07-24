use twilight_model::{
    application::interaction::{InteractionData, application_command::CommandOptionValue},
    channel::message::MessageFlags,
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

    let result = connection::search_planets(
        &input.ok_or(anyhow::anyhow!("No input option provided"))?,
        state.clone(),
    )
    .await;
    let mut error_occurred = false;

    if let Err(e) = &result {
        state
            .client
            .interaction(state.application_id)
            .create_followup(&event.token)
            .content(&format!("An error happened:\n{}", e.to_string()))
            .flags(MessageFlags::EPHEMERAL)
            .await?;
        error_occurred = true
    }
    if error_occurred {
        if let Err(e) = result {
            return Err(e);
        }
    }

    let attachment = result?;
    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .attachments(&[attachment])
        .await?;

    Ok(())
}
