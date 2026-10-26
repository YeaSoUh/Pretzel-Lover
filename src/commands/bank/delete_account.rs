use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::CommandData,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    AppState,
    commands::{
        bank::db::connection::{delete_bank_account, get_bank_account},
        defer, respond,
    },
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    _data: &Box<CommandData>,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, false).await?;

    let user_id = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    let bank_account = get_bank_account(user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("The bank account doesn't exist"))?;

    if bank_account.frozen {
        respond(
            state.clone(),
            &event.token,
            "You are not allowed to delete a bank account",
        )
        .await?;
        return Ok(());
    }

    delete_bank_account(&bank_account).await?;

    respond(
        state.clone(),
        &event.token,
        "Your bank account has been deleted",
    )
    .await?;

    Ok(())
}
