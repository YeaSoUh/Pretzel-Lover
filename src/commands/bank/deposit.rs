use std::{str::FromStr, sync::Arc};

use tracing::instrument;
use twilight_model::application::interaction::{Interaction, application_command::CommandData};

use crate::{AppState, commands::{bank::db::connection::get_bank_account, defer, respond}};

enum Bank {
    NA,
    Aeusvane,
    Iratha,
}

impl FromStr for Bank {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase() {
            name if name.contains("NA") || name.contains("New Astruxia") => Ok(Bank::NA),
            name if name.contains("Auesvane") => Ok(Bank::Aeusvane),
            name if name.contains("Iratha") => Ok(Bank::Iratha),
            _ => Err(anyhow::anyhow!("Invalid bank"))
        }
    }
}

impl std::fmt::Display for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bank::NA => {
                if f.alternate() {
                    write!(f, "NA")?;
                } else {
                    write!(f, "New Astruxia")?;
                }
            }
            Bank::Aeusvane => write!(f, "Aeusvane")?,
            Bank::Iratha => write!(f, "Iratha")?,
        }

        write!(f, " Bank")
    }
}

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