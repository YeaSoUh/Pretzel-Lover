use std::{
    borrow::Cow, sync::{Arc, OnceLock},
};

use tracing::instrument;
use turso::Builder;

use crate::{
    commands::bank::db::internal_types::{Salaries, Salary}, db::{
        helpers::execute,
        types::{DatabaseStruct, DbInstruction},
    },
};

static DB: OnceLock<DatabaseStruct> = OnceLock::new();

#[instrument(err)]
pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url).build().await?;
    DB.set(DatabaseStruct {
        database: Arc::new(db),
    })
    .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    Ok(())
}

#[instrument(err)]
pub async fn get_salary(id: uuid::Uuid) -> anyhow::Result<Option<Salary>> {
    if id.get_version_num() != 7 {
        anyhow::bail!("Invalid UUID version")
    }

    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let mut payments = execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed("SELECT * FROM salaries WHERE id = ?1 LIMIT 1"),
            params: [id.as_bytes().as_slice()],
        },
    )
    .await?
    .into_rows()
    .ok_or(anyhow::anyhow!("Rows weren't found"))?;

    if let Some(row) = payments.next().await? {
        return Ok(Some(row.try_into()?));
    }

    Ok(None)
}

#[instrument(err)]
pub async fn get_coming_salary(id: uuid::Uuid) -> anyhow::Result<Option<Salary>> {
    if id.get_version_num() != 7 {
        anyhow::bail!("Invalid UUID version")
    }

    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let mut payments = execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed("SELECT * FROM salaries ORDER BY run_at ASC LIMIT 1"),
            params: [id.as_bytes().as_slice()],
        },
    )
    .await?
    .into_rows()
    .ok_or(anyhow::anyhow!("Rows weren't found"))?;

    if let Some(row) = payments.next().await? {
        return Ok(Some(row.try_into()?));
    }

    Ok(None)
}

#[instrument(err)]
pub async fn remove_salary(id: uuid::Uuid) -> anyhow::Result<()> {
    if id.get_version_num() != 7 {
        anyhow::bail!("Invalid UUID version")
    }

    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed("DELETE FROM salaries WHERE id = ?1"),
            params: [id.as_bytes().as_slice()],
        },
    )
    .await?;

    Ok(())
}

#[instrument(err)]
pub async fn edit_salary(query: Salary) -> anyhow::Result<()> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let (sql, query_values) = {
        let columns = vec![Salaries::Id, Salaries::From, Salaries::To, Salaries::Amount, Salaries::IntervalHours, Salaries::RunAt];
        let values: Vec<sea_query::SimpleExpr> = vec![query.id.as_slice().into(), query.from.get().into(), query.to.get().into(), query.amount.into(), query.interval_hours.into(), query.run_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true).into()];

        sea_query::Query::insert()
            .into_table(Salaries::Table)
            .columns(columns)
            .values(values)?
            .build(sea_query::SqliteQueryBuilder)
    };

    let turso_params: Vec<turso::Value> = query_values
    .into_iter()
    .map(|v| {
        Ok(match v {
            sea_query::Value::Int(Some(i)) => turso::Value::Integer(i as i64),
            sea_query::Value::BigInt(Some(i)) => turso::Value::Integer(i),
            sea_query::Value::BigUnsigned(Some(i)) => turso::Value::Integer(i64::try_from(i)?),
            sea_query::Value::Double(Some(f)) => turso::Value::Real(f),
            sea_query::Value::String(Some(s)) => turso::Value::Text(s),
            sea_query::Value::Bool(Some(b)) => turso::Value::Integer(b as i64),
            sea_query::Value::Bytes(Some(b)) => turso::Value::Blob(b),
            other => anyhow::bail!("unsupported sea query value:\n{other:?}"),
        })
    })
    .collect::<anyhow::Result<_>>()?;

    execute(
        db,
        DbInstruction::Execute {
            execute: sql.into(),
            params: turso_params,
        },
    )
    .await?;

    Ok(())
}