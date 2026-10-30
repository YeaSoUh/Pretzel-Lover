use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::{Interaction, application_command::CommandData}
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
    event: Interaction,
    _data: CommandData,
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

    let user_id = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    if state.configs.users_blacklist.contains(&user_id) {
        respond(
            state.clone(),
            &event.token,
            "You are blacklisted from making a bank account",
        )
        .await?;
    }

    let bank_account = BankAccount {
        account_id: user_id,
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
