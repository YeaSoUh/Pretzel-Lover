// it's empty
// edit: now it's not

use std::sync::Arc;

use tracing::instrument;
use twilight_model::application::interaction::{
    Interaction,
    application_command::{CommandData, CommandOptionValue},
};
use uuid::Uuid;

use crate::{
    AppState,
    commands::{
        bank::db::{connection::remove_salary_instruction, internal_types::DbTransaction},
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

    if data.options.is_empty() {
        anyhow::bail!("No options")
    }

    let mut id = None;

    for option in &data.options {
        match (&*option.name, &option.value) {
            ("id", CommandOptionValue::String(id2)) => {
                id = Some(id2.clone());
            }
            _ => {}
        }
    }
    let id = match id {
        Some(id2) => id2,
        None => {
            respond(state.clone(), &event.token, "No provided id").await?;

            return Ok(());
        }
    };

    let result = DbTransaction::new(vec![
        remove_salary_instruction(Uuid::parse_str(&id)?).await?,
    ])
    .run()
    .await;

    if let Err(e) = result {
        respond(
            state.clone(),
            &event.token,
            "Invalid salary id or another error",
        )
        .await?;

        return Err(e);
    }

    respond(
        state.clone(),
        &event.token,
        "Salary has been deleted successfully",
    )
    .await?;

    Ok(())
}
