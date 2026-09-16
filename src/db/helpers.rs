use std::{borrow::Cow, sync::OnceLock, time::Duration};
use tracing::instrument;
use turso::{Connection, Error, IntoParams, Rows};

use crate::{AppState, db::connection::Checks};

pub async fn execute(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams + Clone,
) -> anyhow::Result<()> {
    let max_attempts = 10;
    let mut attempts = 1;

    loop {
        if attempts > max_attempts {
            return Err(anyhow::anyhow!("Reached max attempts"));
        }
        conn.execute("BEGIN CONCURRENT", ()).await?;
        match conn.execute(sql, params.clone()).await {
            Ok(_) => {
                conn.execute("COMMIT", ()).await?;
                return Ok(());
            }
            Err(e) if is_retryable(&e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                tracing::warn!(?e, "Retrying request");
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(15)).await;
                continue;
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(e.into());
            }
        }
    }
}

#[instrument(skip_all, err)]
pub async fn query(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams + Clone,
) -> anyhow::Result<Rows> {
    let max_attempts = 10;
    let mut attempts = 1;

    loop {
        if attempts > max_attempts {
            return Err(anyhow::anyhow!("Reached max attempts"));
        }
        conn.execute("BEGIN CONCURRENT", ()).await?;
        match conn.query(sql, params.clone()).await {
            Ok(r) => {
                conn.execute("COMMIT", ()).await?;
                return Ok(r);
            }
            Err(e) if is_retryable(&e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                tracing::warn!(?e, "Retrying request");
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(15)).await;
                continue;
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(e.into());
            }
        }
    }
}

#[allow(non_snake_case)]
pub fn validate(key: &str, value: &str, CHECKS: &OnceLock<Checks>) -> anyhow::Result<()> {
    match key {
        "malachite" | "hematite" | "petroleum" | "coal" | "gummite" | "tektite" | "bauxite"
        | "gold" | "cerussite" => {
            let concentration = value.parse::<f64>()?;
            if concentration < 0.0 || concentration > 3.0 {
                anyhow::bail!("Wrong concentration information in {}", key)
            }
        }
        "life" | "lime" | "saltpeter" | "quartz" | "ice" => {
            if value != "true" && value != "false" {
                anyhow::bail!("{} is supposed to have true/false value", key)
            }
        }
        // will be improved later
        "sector" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .sectors
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Sector type '{}' isn't in the game", value))
            }
        }
        "tectonics" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .tectonics
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tectonic type '{}' isn't in the game", value))
            }
        }
        "atmosphere" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .atmospheres
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Atmospheric type '{}' isn't in the game", value))
            }
        }
        /*"oceans" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .oceans
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Oceans type '{}' isn't in the game", value))
            }
        }*/
        /*"trees" | "sub trees" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .trees
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tree type '{}' isn't in the game", value))
            }
        }*/
        _ => {}
    }

    Ok(())
}

pub fn check_sql(input: &str, state: AppState) -> bool {
    let upper = input.to_uppercase();
    state
        .configs
        .sql_blacklist
        .iter()
        .any(|sql| upper.contains(sql))
}

pub fn check_index(index: &str) -> anyhow::Result<()> {
    match index.split_once("-") {
        Some((num1, num2)) => {
            num1.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("No star id"))?;
            if let Err(_) = num2.parse::<i64>() {
                let mut split = num2.split("-");
                split
                    .by_ref()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No planet id"))?
                    .parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Blacklisted sql"))?;
                split
                    .by_ref()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No moon id"))?
                    .parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Blacklisted sql"))?;

                if split.next().is_some() {
                    anyhow::bail!("Blacklisted sql")
                }
            }
            Ok(())
        }
        None => anyhow::bail!("Blacklisted sql"),
    }
}

pub fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains("&&") || input.contains("||") {
        Cow::Owned(input.replace("&&", "and").replace("||", "or"))
    } else {
        Cow::Borrowed(input)
    }
}

// taken from official turso docs
pub fn is_retryable(e: &Error) -> bool {
    matches!(e, Error::Busy(_) | Error::BusySnapshot(_))
        || matches!(e, Error::Error(msg) if msg.contains("conflict"))
}
