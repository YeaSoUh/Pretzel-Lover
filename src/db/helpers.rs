use std::time::Duration;
use tracing::instrument;
use turso::Error;

use crate::db::types::{DatabaseStruct, DbInstruction, DbResults};

const MAX_ATTEMPTS: u64 = 10;

#[instrument(skip_all, err)]
pub async fn execute(
    db: &DatabaseStruct,
    instructions: &[DbInstruction],
) -> anyhow::Result<DbResults> {
    let conn = db.get_conn().await?;

    for attempt in 1..=MAX_ATTEMPTS {
        conn.execute("BEGIN CONCURRENT", ()).await?;
        let mut results = Vec::with_capacity(instructions.len());
        let mut failure = None;

        for dbinst in instructions {
            match dbinst.run(&conn).await {
                Ok(dbres) => results.push(dbres),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }

        match failure {
            None => {
                match conn.execute("COMMIT", ()).await {
                    Ok(_) => return Ok(DbResults::new(results)),
                    Err(e) if is_retryable(&e) => {
                        let _ = conn.execute("ROLLBACK", ()).await;
                        tracing::warn!(?e, attempt, "Commit conflict, retrying");
                    }
                    Err(e) => {
                        let _ = conn.execute("ROLLBACK", ()).await;
                        return Err(e.into());
                    }
                }
            }
            Some(e) if is_retryable(&e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                tracing::warn!(?e, attempt, "Retrying request");
            }
            Some(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(e.into());
            }
        }

        tokio::time::sleep(Duration::from_millis(15 * attempt)).await;
    }

    Err(anyhow::anyhow!("Reached max attempts ({})", MAX_ATTEMPTS))
}

// taken from the official turso docs
pub fn is_retryable(e: &Error) -> bool {
    matches!(e, Error::Busy(_) | Error::BusySnapshot(_))
        || matches!(e, Error::Error(msg) if msg.contains("conflict"))
}
