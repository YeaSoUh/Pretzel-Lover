use std::{borrow::Cow, sync::Arc, time::Duration};

use turso::{Connection, Database, IntoParams, Rows};

pub struct DatabaseStruct {
    pub(crate) database: Arc<Database>,
}

impl DatabaseStruct {
    pub async fn get_conn(&self) -> anyhow::Result<Connection> {
        let db_ref = Arc::clone(&self.database);

        let conn = db_ref.as_ref().connect()?;
        conn.busy_timeout(Duration::from_millis(500))?; // 0.5 seconds
        conn.pragma_update("journal_mode", "'mvcc'").await?; // enables concurrency writes which is good!

        Ok(conn)
    }
}

pub enum DbResult {
    #[allow(dead_code)]
    Execute(u64),
    Query(Rows),
}

impl DbResult {
    #[allow(dead_code)]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            DbResult::Execute(num) => Some(*num),
            _ => None,
        }
    }
    pub fn into_rows(self) -> Option<Rows> {
        match self {
            DbResult::Query(rows) => Some(rows),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DbInstruction<P: IntoParams> {
    Execute {
        execute: Cow<'static, str>,
        params: P,
    },
    Query {
        query: Cow<'static, str>,
        params: P,
    },
}

impl<P: IntoParams> DbInstruction<P> {
    pub async fn run(self, conn: &Connection) -> Result<DbResult, turso::Error> {
        Ok(match self {
            DbInstruction::Execute { execute, params } => {
                DbResult::Execute(conn.execute(execute, params).await?)
            }
            DbInstruction::Query { query, params } => {
                DbResult::Query(conn.query(query, params).await?)
            }
        })
    }
}
