use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
};

use tracing::instrument;
use turso::Builder;
use twilight_model::id::{Id, marker::UserMarker};

use crate::{
    commands::bank::db::internal_types::{BankAccount, BankAccounts, Salaries, Salary},
    db::{
        helpers::execute,
        types::{DatabaseStruct, DbInstruction},
    },
};

static DB: OnceLock<DatabaseStruct> = OnceLock::new();

#[instrument(err)]
pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url)
        .experimental_custom_types(true)
        .build()
        .await?;
    DB.set(DatabaseStruct {
        database: Arc::new(db),
    })
    .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed(
                "CREATE TABLE IF NOT EXISTS bank_accounts (
            account_id INTEGER NOT NULL PRIMARY KEY,
            amount     INTEGER NOT NULL,
            frozen     BOOLEAN NOT NULL
            ) STRICT;",
            ),
            params: (),
        },
    )
    .await?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed(
                "CREATE TABLE IF NOT EXISTS salaries (
            id             BLOB NOT NULL PRIMARY KEY CHECK (length(id) = 16),
            from_account   INTEGER NOT NULL,
            to_account     INTEGER NOT NULL,
            amount         INTEGER NOT NULL CHECK (amount > 0),
            interval_hours INTEGER NOT NULL CHECK (interval_hours > 0),
            run_at         TIMESTAMP NOT NULL
            ) STRICT;",
            ),
            params: (),
        },
    )
    .await?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed(
                "CREATE INDEX IF NOT EXISTS salaries_run_at ON salaries (run_at);",
            ),
            params: (),
        },
    )
    .await?;

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

    let payments = execute(
        &db,
        DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM salaries WHERE id = ?1 LIMIT 1"),
            params: [id.as_bytes().as_slice()],
        },
    )
    .await?
    .into_rows();

    if let Some(mut payments) = payments {
        if let Some(row) = payments.next().await? {
            return Ok(Some(row.try_into()?));
        }
    }

    Ok(None)
}

#[instrument(err)]
pub async fn get_coming_salary() -> anyhow::Result<Option<Salary>> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let payments = execute(
        &db,
        DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM salaries ORDER BY run_at ASC LIMIT 1"),
            params: (),
        },
    )
    .await?
    .into_rows();

    if let Some(mut payments) = payments {
        if let Some(row) = payments.next().await? {
            return Ok(Some(row.try_into()?));
        }
    }

    Ok(None)
}

// i will make a paginator later
// or not i thought of an idea without using cache but still have paginator style for a user
// this might stay the same if my plan goes correctly
#[instrument(err)]
pub async fn search_salaries(
    bank_account: &BankAccount,
    page: u32,
) -> anyhow::Result<Option<Vec<Salary>>> {
    if bank_account.frozen {
        anyhow::bail!("Frozen account")
    }

    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let per_page = 10;
    let offset = (page - 1) * per_page;

    let payments = execute(
        &db,
        DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM salaries WHERE from = ?1 LIMIT ?2 OFFSET ?3"),
            params: [
                bank_account.account_id.get(),
                per_page as u64,
                offset as u64,
            ],
        },
    )
    .await?
    .into_rows();

    if let Some(mut payments) = payments {
        let mut salaries: Vec<Salary> = Vec::new();
        while let Some(row) = payments.next().await? {
            salaries.push(row.try_into()?);
        }

        return Ok(Some(salaries));
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
pub async fn edit_salary(query: &Salary) -> anyhow::Result<()> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let (sql, query_values) = {
        let columns = vec![
            Salaries::Id,
            Salaries::FromAccount,
            Salaries::ToAccount,
            Salaries::Amount,
            Salaries::IntervalHours,
            Salaries::RunAt,
        ];
        let values: Vec<sea_query::SimpleExpr> = vec![
            query.id.as_slice().into(),
            query.from_account.get().into(),
            query.to_account.get().into(),
            query.amount.into(),
            query.interval_hours.into(),
            query
                .run_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                .into(),
        ];

        let conflict = sea_query::OnConflict::columns([Salaries::RunAt]);

        sea_query::Query::insert()
            .into_table(Salaries::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(conflict)
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

#[instrument(err)]
pub async fn edit_bank_account(bank_account: &BankAccount) -> anyhow::Result<()> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let (sql, query_values) = {
        let columns = vec![
            BankAccounts::AccountId,
            BankAccounts::Amount,
            BankAccounts::Frozen,
        ];
        let values: Vec<sea_query::SimpleExpr> = vec![
            bank_account.account_id.get().into(),
            bank_account.amount.into(),
            bank_account.frozen.into(),
        ];

        let conflict = sea_query::OnConflict::columns([BankAccounts::Amount, BankAccounts::Frozen]);

        sea_query::Query::insert()
            .into_table(BankAccounts::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(conflict)
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

#[instrument(err)]
pub async fn get_bank_account(account_id: Id<UserMarker>) -> anyhow::Result<Option<BankAccount>> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let accounts = execute(
        &db,
        DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM bank_accounts WHERE id = ?1 LIMIT 1"),
            params: [account_id.get()],
        },
    )
    .await?
    .into_rows();

    if let Some(mut accounts) = accounts {
        if let Some(row) = accounts.next().await? {
            return Ok(Some(row.try_into()?));
        }
    }

    Ok(None)
}

#[instrument(err)]
pub async fn delete_bank_account(bank_account: &BankAccount) -> anyhow::Result<()> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed("DELETE FROM bank_accounts WHERE id = ?1"),
            params: [bank_account.account_id.get()],
        },
    )
    .await?;

    execute(
        &db,
        DbInstruction::Execute {
            execute: Cow::Borrowed(r#"DELETE FROM salaries WHERE "from" = ?1 OR "to" = ?1"#),
            params: [bank_account.account_id.get()],
        },
    )
    .await?;

    Ok(())
}
