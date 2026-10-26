use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    AppState,
    commands::{bank::db::internal_types::Payment, defer, respond},
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, false).await?;

    if event
        .channel
        .as_ref()
        .is_none_or(|chn| !state.configs.allowed_channels.contains(&chn.id))
    {
        respond(state.clone(), &event.token, "Outside of allowed channels").await?;
        return Ok(());
    }

    let user_id_from = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    let mut user_id_to = None;
    let mut amount_to_transfer = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options were provided")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("user", CommandOptionValue::User(user_id)) => {
                user_id_to = Some(user_id.clone());
            }
            ("amount", CommandOptionValue::Integer(amount)) => {
                amount_to_transfer = Some(amount.clone());
            }
            _ => {}
        }
    }
    let user_id_to = user_id_to.ok_or_else(|| anyhow::anyhow!("Missing user option"))?;
    let amount_to_transfer =
        amount_to_transfer.ok_or_else(|| anyhow::anyhow!("Missing amount option"))?;

    let payment = Payment {
        from_account: user_id_from,
        to_account: user_id_to,
        amount: amount_to_transfer,
    };

    payment.process_payment().await?;

    respond(
        state.clone(),
        &event.token,
        "Transaction has been completed successfully",
    )
    .await?;

    Ok(())
}
