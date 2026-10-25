use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    AppState,
    commands::{
        bank::db::{
            connection::{edit_bank_account, get_bank_account},
            internal_types::BankAccount,
        },
        defer, respond,
    },
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, true).await?;

    let mut user_id = None;
    let mut amount_to_add = None;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("user_id", CommandOptionValue::User(user_id2)) => {
                user_id = Some(user_id2.clone());
            }
            ("amount", CommandOptionValue::Integer(amount)) => {
                amount_to_add = Some(amount.clone());
            }
            _ => {}
        }
    }
    let user_id = user_id.ok_or_else(|| anyhow::anyhow!("Missing user option"))?;
    let amount_to_add = amount_to_add.ok_or_else(|| anyhow::anyhow!("Missing amount option"))?;

    let old_bank_account = get_bank_account(user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("The bank account doesn't exist"))?;

    let new_bank_account = BankAccount {
        account_id: user_id,
        amount: amount_to_add + old_bank_account.amount,
        frozen: old_bank_account.frozen,
    };
    edit_bank_account(&new_bank_account).await?;

    respond(
        state.clone(),
        &event.token,
        "Transaction has been completed successfully",
    )
    .await?;

    Ok(())
}
