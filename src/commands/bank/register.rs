use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::CommandData,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    AppState,
    commands::{
        bank::db::{connection::edit_bank_account, internal_types::BankAccount},
        defer, respond,
    },
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    _data: &Box<CommandData>,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, true).await?;

    if state.configs.users_blacklist.contains(
        &event
            .author_id()
            .ok_or_else(|| anyhow::anyhow!("No user id in author id function"))?,
    ) {
        respond(
            state.clone(),
            &event.token,
            "You are blacklisted from making a bank account",
        )
        .await?;
    }

    let bank_account = BankAccount {
        account_id: event
            .author_id()
            .ok_or_else(|| anyhow::anyhow!("No user id in author id function"))?,
        amount: 0,
        frozen: false,
    };

    edit_bank_account(&bank_account).await?;

    respond(
        state.clone(),
        &event.token,
        "Your bank account has been created!",
    )
    .await?;

    Ok(())
}
