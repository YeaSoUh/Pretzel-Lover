// it's empty
// edit: now it's not

use std::sync::Arc;

use tracing::instrument;
use twilight_model::application::interaction::{
    Interaction,
    application_command::{CommandData, CommandOptionValue},
};

use crate::{
    AppState,
    commands::{
        bank::db::{
            connection::edit_salary_instruction,
            internal_types::{DbTransaction, Salary},
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

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    let mut user_id_to = None;
    let mut amount_to_transfer = None;
    let mut interval_hours = None;

    for option in &data.options {
        match (&*option.name, &option.value) {
            ("user", CommandOptionValue::User(user_id)) => {
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
    let user_id_to = match user_id_to {
        Some(user_id) => user_id,
        None => {
            respond(
                state.clone(),
                &event.token,
                "Missing user to send money",
            ).await?;

            return Ok(());
        }
    };
    let amount_to_transfer = match amount_to_transfer {
        Some(amount) => amount,
        None => {
            respond(
                state.clone(),
                &event.token,
                "Missing amount to send",
            ).await?;

            return Ok(());
        }
    };
    let interval_hours = match interval_hour {
        Some(interval) => interval,
        None => {
            respond(
                state.clone(),
                &event.token,
                "Missing interval in hours",
            ).await?;

            return Ok(());
        }
    };
    

    if user_id_from == user_id_to {
        respond(
            state.clone(),
            &event.token,
            "Can't give yourself money",
        ).await?;

        return Ok(());
    }
    if amount_to_transfer < 0 {
        respond(
            state.clone(),
            &event.token,
            "Can't have negative amount",
        ).await?;

        return Ok(());
    }
    if interval_hours < 0 {
        respond(
            state.clone(),
            &event.token,
            "Can't have negative time",
        ).await?;

        return Ok(());
    }

    let salary = Salary {
        id: *uuid::Uuid::now_v7().as_bytes(),
        from_account: user_id_from,
        to_account: user_id_to,
        interval_hours: interval_hours,
        amount: amount_to_transfer,
        run_at: chrono::Utc::now() + chrono::Duration::hours(interval_hours),
    };

    DbTransaction::new(vec![edit_salary_instruction(&salary).await?])
        .run()
        .await?;

    respond(
        state.clone(),
        &event.token,
        "Salary has been created successfully",
    )
    .await?;

    Ok(())
}
