use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::{Interaction, application_command::{CommandData, CommandOptionValue}}, id::Id,
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
    event: Interaction,
    data: CommandData,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, true).await?;

    let user = event
        .member
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not a member"))?;
    let user_roles = &user.roles;

    if !user_roles.contains(&Id::new(1410929363863732234)) // co-owner
        && !user_roles.contains(&Id::new(1410929110502608896))
    // owner
    {
        respond(state.clone(), &event.token, "Not allowed to run command").await?;
        return Ok(()); // just to suppress error
    }

    let mut user_id = None;
    let mut amount_to_add = None;

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    for option in &data.options {
        match (&*option.name, &option.value) {
            ("user", CommandOptionValue::User(user_id2)) => {
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

    if old_bank_account.frozen {
        anyhow::bail!("The account is frozen")
    }

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
