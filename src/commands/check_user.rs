use std::sync::Arc;

use tracing::instrument;
use twilight_model::{application::interaction::application_command::{CommandData, CommandOptionValue}, channel::message::MessageFlags, gateway::payload::incoming::InteractionCreate, id::Id};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{AppState, commands::{self, response}};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    commands::defer(state.clone(), &event, true).await?;

    let user = event
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("Not a member"))?;
    let user_roles = &user.roles;

    if !user_roles.contains(&Id::new(1410929363863732234)) // co-owner
        && !user_roles.contains(&Id::new(1410929110502608896)) // owner
    {
        state
            .client
            .interaction(state.application_id)
            .create_response(
                event.id,
                &event.token,
                &response(
                    InteractionResponseDataBuilder::new()
                        .content("Not allowed to run command")
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            )
            .await?;
        return Ok(()); // just to suppress error
    }

    let mut user_id: Option<String> = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("user_id", CommandOptionValue::String(user_id2)) => {
                user_id = Some(user_id2.clone());
            }
            _ => {}
        }
    }
    let user_id = user_id.unwrap();

    let client = reqwest::Client::new();

    let request = client
        .get(format!("https://api.simmer.icu/v2/checkuser/{}", user_id))
        .header("Authorization", format!("Bearer {}", state.configs.simmer_api_key));

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .content(&request.send().await?.text().await?)
        .await?;


    Ok(())
}