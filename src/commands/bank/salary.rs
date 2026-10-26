// it's empty
// edit: now it's not

use std::sync::Arc;

use tracing::instrument;
use twilight_model::{application::interaction::application_command::{CommandData, CommandOptionValue}, gateway::payload::incoming::InteractionCreate};

use crate::{AppState, commands::{bank::db::{connection::edit_salary, internal_types::Salary}, defer, respond}};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: &Box<InteractionCreate>,
    data: &Box<CommandData>,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, true).await?;

    let user_id_from = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    let cmd = data.as_ref();
    if cmd.options.is_empty() {
        anyhow::bail!("No options")
    }

    let mut user_id_to = None;
    let mut amount_to_transfer = None;
    let mut interval_hours = None;

    for option in &cmd.options {
        match (&*option.name, &option.value) {
            ("user_id", CommandOptionValue::User(user_id)) => {
                user_id_to = Some(user_id.clone());
            }
            ("amount", CommandOptionValue::Integer(amount)) => {
                amount_to_transfer = Some(amount.clone());
            }
            ("interval_hours", CommandOptionValue::Integer(interval_hours2)) => {
                interval_hours = Some(interval_hours2.clone());
            }
            _ => {}
        }
    }
    let user_id_to = user_id_to.ok_or_else(|| anyhow::anyhow!("Missing user option"))?;
    let amount_to_transfer =
        amount_to_transfer.ok_or_else(|| anyhow::anyhow!("Missing amount option"))?;
    let interval_hours = interval_hours.ok_or_else(|| anyhow::anyhow!("Missing interval_hours option"))?;

    let salary = Salary {
        id: *uuid::Uuid::now_v7().as_bytes(),
        from_account: user_id_from,
        to_account: user_id_to,
        interval_hours: interval_hours,
        amount: amount_to_transfer,
        run_at: chrono::Utc::now() + chrono::Duration::hours(interval_hours),
    };

    edit_salary(&salary).await?;

    respond(state.clone(), &event.token, "Salary has been created successfully").await?;

    Ok(())
}