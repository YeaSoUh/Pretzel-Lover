use std::{borrow::Cow, ops::{Deref, DerefMut}, sync::Arc, time::Duration};

use turso::{Connection, Database, Rows, Value};

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
    Execute(u64),
    Query(Rows),
}

impl DbResult {
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

pub struct DbResults {
    results: Vec<DbResult>,
}

impl DbResults {
    pub fn new(results: Vec<DbResult>) -> Self {
        Self {
            results,
        }
    }
    
    pub fn into_inner(self) -> Vec<DbResult> {
        self.results
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.first()?.as_u64()
    }
    
    pub fn into_rows(self) -> Option<Rows> {
        self.results.into_iter().next()?.into_rows()
    }
}

impl Deref for DbResults {
    type Target = Vec<DbResult>;

    fn deref(&self) -> &Self::Target {
        &self.results
    }
}

impl DerefMut for DbResults {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.results
    }
}

#[derive(Debug, Clone)]
pub enum DbInstruction {
    Execute {
        execute: Cow<'static, str>,
        params: Vec<Value>,
    },
    Query {
        query: Cow<'static, str>,
        params: Vec<Value>,
    },
}

impl DbInstruction {
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
