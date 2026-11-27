use std::time::Duration;
use tracing::instrument;
use turso::{Connection, Error, transaction::TransactionBehavior};

use crate::db::types::{DbInstruction, DbResults};

const MAX_ATTEMPTS: u64 = 10;

#[instrument(skip_all, err)]
pub async fn execute(
    conn: &mut Connection,
    instructions: &[DbInstruction],
) -> anyhow::Result<DbResults> {
    for attempt in 1..=MAX_ATTEMPTS {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Concurrent).await?;
        let mut results = Vec::with_capacity(instructions.len());
        let mut failure = None;

        for dbinst in instructions {
            match dbinst.run_tx(&tx).await {
                Ok(dbres) => results.push(dbres),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }

        match failure {
            None => match tx.commit().await {
                Ok(_) => return Ok(DbResults::new(results)),
                Err(e) if is_retryable(&e) => {
                    tracing::warn!(?e, attempt, "Commit conflict, retrying");
                }
                Err(e) => {
                    return Err(e.into());
                }
            },
            Some(e) if is_retryable(&e) => {
                tracing::warn!(?e, attempt, "Retrying request");
            }
            Some(e) => {
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
