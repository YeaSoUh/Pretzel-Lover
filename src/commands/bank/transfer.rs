use std::sync::Arc;

use tracing::instrument;
use twilight_model::{
    application::interaction::{
        Interaction,
        application_command::{CommandData, CommandOptionValue},
    },
    channel::message::MessageFlags,
};

use crate::{
    AppState,
    commands::{
        bank::db::internal_types::{ComponentV2Logger, Payment},
        defer, respond,
    },
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: Interaction,
    data: CommandData,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, event.authorizing_integration_owners.guild.is_none()).await?;

    let user_id_from = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    let mut user_id_to = None;
    let mut amount_to_transfer = None;
    let mut reason = None;

    if data.options.is_empty() {
        anyhow::bail!("No options were provided")
    }

    for option in &data.options {
        match (&*option.name, &option.value) {
            ("user", CommandOptionValue::User(user_id)) => {
                user_id_to = Some(user_id.clone());
            }
            ("amount", CommandOptionValue::Integer(amount)) => {
                amount_to_transfer = Some(amount.clone());
            }
            ("reason", CommandOptionValue::String(reason2)) => {
                reason = Some(reason2.clone());
            }
            _ => {}
        }
    }
    let user_id_to = user_id_to.ok_or_else(|| anyhow::anyhow!("Missing user option"))?;
    let amount_to_transfer =
        amount_to_transfer.ok_or_else(|| anyhow::anyhow!("Missing amount option"))?;
    reason = reason.filter(|x| !x.is_empty());

    if user_id_from == user_id_to {
        anyhow::bail!("Can't give yourself money")
    }
    if amount_to_transfer <= 0 {
        anyhow::bail!("Can't have negative amount")
    }

    let payment = Payment {
        from_account: user_id_from,
        to_account: user_id_to,
        amount: amount_to_transfer,
        reason: reason,
    };

    let result = payment.process_payment().await;

    if let Err(e) = result {
        respond(state.clone(), &event.token, &format!("{}", e.to_string())).await?;

        return Err(e);
    }

    state
        .client
        .create_message(state.configs.logging_channel)
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(&[payment.to_component()])
        .await?;

    respond(
        state.clone(),
        &event.token,
        "Transaction has been completed successfully",
    )
    .await?;

    Ok(())
}
