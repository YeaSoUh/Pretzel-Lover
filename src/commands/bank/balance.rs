use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::{Interaction, application_command::{CommandData, CommandOptionValue}}
};

use crate::{
    AppState,
    commands::{bank::db::connection::get_bank_account, defer, respond},
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: Interaction,
    data: CommandData,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, false).await?;

    let author_id = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    if event
        .channel
        .as_ref()
        .is_none_or(|chn| !state.configs.allowed_channels.contains(&chn.id))
    {
        respond(state.clone(), &event.token, "Outside of allowed channels").await?;
        return Ok(());
    }

    let mut user_id = None;

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &data.options {
        match (&*option.name, &option.value) {
            ("user", CommandOptionValue::User(user_id2)) => {
                user_id = Some(user_id2.clone());
            }
            _ => {}
        }
    }
    let user_id = match user_id {
        Some(user_id2) => user_id2,
        None => author_id,
    };

    let bank_account = get_bank_account(user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Bank account wasn't found"))?;

    respond(
        state.clone(),
        &event.token,
        &format!("Balance of the user is {}", bank_account.amount),
    )
    .await?;

    Ok(())
}
