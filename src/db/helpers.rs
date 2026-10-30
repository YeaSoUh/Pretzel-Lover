use std::time::Duration;
use tracing::instrument;
use turso::Error;

use crate::db::types::{DatabaseStruct, DbInstruction, DbResults};

const MAX_ATTEMPTS: u64 = 10;

#[instrument(skip_all, err)]
pub async fn execute(
    db: &DatabaseStruct,
    instruction: DbInstruction,
) -> anyhow::Result<DbResults> {
    let conn = db.get_conn().await?;

    let mut attempts = 1;

    loop {
        if attempts > MAX_ATTEMPTS {
            return Err(anyhow::anyhow!("Reached max attempts"));
        }
        conn.execute("BEGIN CONCURRENT", ()).await?;
        match instruction.clone().run(&conn).await {
            Ok(dbres) => {
                conn.execute("COMMIT", ()).await?;
                return Ok(
                    DbResults::new(vec![dbres])
                );
            }
            Err(e) if is_retryable(&e) => {
                conn.execute("ROLLBACK", ()).await?;
                tracing::warn!(?e, "Retrying request");
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(15)).await;
                continue;
            }
            Err(e) => {
                conn.execute("ROLLBACK", ()).await?;
                return Err(e.into());
            }
        }
    }
}

// taken from official turso docs
pub fn is_retryable(e: &Error) -> bool {
    matches!(e, Error::Busy(_) | Error::BusySnapshot(_))
        || matches!(e, Error::Error(msg) if msg.contains("conflict"))
}
