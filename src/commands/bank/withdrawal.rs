use std::sync::Arc;

use tracing::instrument;
use twilight_model::application::interaction::{Interaction, application_command::CommandData};

use crate::{AppState, commands::{bank::db::connection::get_bank_account, defer, respond}};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: Interaction,
    data: CommandData,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, event.authorizing_integration_owners.guild.is_none()).await?;

    let user_id = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    let bank_account = match get_bank_account(user_id).await? {
        Some(bank) => bank,
        None => {
            respond(
                state.clone(),
                &event.token,
                "You have no bank account registered",
            )
            .await?;

            return Ok(());
        }
    };

    if bank_account.frozen {
        respond(
            state.clone(),
            &event.token,
            "You are not allowed to run this command",
        )
        .await?;
        return Ok(());
    }

    Ok(())
}