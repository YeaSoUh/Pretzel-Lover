use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
};

use sea_query::ExprTrait;
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

    Ok(())
}

pub fn get_db() -> anyhow::Result<&'static DatabaseStruct> {
    Ok(DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?)
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
        vec![DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM salaries WHERE id = ?1 LIMIT 1"),
            params: vec![id.as_bytes().as_slice().into()],
        }]
        .as_slice(),
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
        vec![DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM salaries ORDER BY run_at ASC LIMIT 1"),
            params: Vec::new(),
        }]
        .as_slice(),
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
        vec![DbInstruction::Query {
            query: Cow::Borrowed(
                "SELECT * FROM salaries WHERE from_account = ?1 LIMIT ?2 OFFSET ?3",
            ),
            params: vec![
                (bank_account.account_id.get() as i64).into(),
                per_page.into(),
                offset.into(),
            ],
        }]
        .as_slice(),
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
pub async fn remove_salary_instruction(id: uuid::Uuid) -> anyhow::Result<DbInstruction> {
    if id.get_version_num() != 7 {
        anyhow::bail!("Invalid UUID version")
    }

    Ok(DbInstruction::Execute {
        execute: Cow::Borrowed("DELETE FROM salaries WHERE id = ?1"),
        params: vec![id.as_bytes().as_slice().into()],
    })
}

#[instrument(err)]
pub async fn edit_salary_instruction(query: &Salary) -> anyhow::Result<DbInstruction> {
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
            query.id.as_bytes().as_slice().into(),
            query.from_account.get().into(),
            query.to_account.get().into(),
            query.amount.into(),
            query.interval_hours.into(),
            query
                .run_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                .into(),
        ];

        sea_query::Query::insert()
            .into_table(Salaries::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(
                sea_query::OnConflict::column(Salaries::Id)
                    .update_column(Salaries::RunAt)
                    .to_owned(),
            )
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

    Ok(DbInstruction::Execute {
        execute: sql.into(),
        params: turso_params,
    })
}

#[instrument(err)]
pub async fn edit_bank_account_instruction(
    bank_account: &BankAccount,
) -> anyhow::Result<DbInstruction> {
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

        sea_query::Query::insert()
            .into_table(BankAccounts::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(
                sea_query::OnConflict::column(BankAccounts::AccountId)
                    .update_columns([BankAccounts::Frozen, BankAccounts::Amount])
                    .to_owned(),
            )
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

    Ok(DbInstruction::Execute {
        execute: sql.into(),
        params: turso_params,
    })
}

#[instrument(err)]
pub async fn get_bank_account(account_id: Id<UserMarker>) -> anyhow::Result<Option<BankAccount>> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?;

    let accounts = execute(
        &db,
        vec![DbInstruction::Query {
            query: Cow::Borrowed("SELECT * FROM bank_accounts WHERE account_id = ?1 LIMIT 1"),
            params: vec![(account_id.get() as i64).into()],
        }]
        .as_slice(),
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
pub async fn delete_bank_account_instruction(
    bank_account: &BankAccount,
) -> anyhow::Result<Vec<DbInstruction>> {
    Ok(vec![
        DbInstruction::Execute {
            execute: Cow::Borrowed("DELETE FROM bank_accounts WHERE account_id = ?1"),
            params: vec![(bank_account.account_id.get() as i64).into()],
        },
        DbInstruction::Execute {
            execute: Cow::Borrowed(
                "DELETE FROM salaries WHERE from_account = ?1 OR to_account = ?1",
            ),
            params: vec![(bank_account.account_id.get() as i64).into()],
        },
    ])
}

#[instrument(err)]
pub async fn transfer_pretzels_instruction(
    from: &BankAccount,
    to: &BankAccount,
    amount: i64,
) -> anyhow::Result<Vec<DbInstruction>> {
    let ((from_sql, from_query_values), (to_sql, to_query_values)) = {
        let columns = vec![
            BankAccounts::AccountId,
            BankAccounts::Amount,
            BankAccounts::Frozen,
        ];
        let values: Vec<sea_query::SimpleExpr> = vec![
            from.account_id.get().into(),
            from.amount.into(),
            from.frozen.into(),
        ];

        let from_query = sea_query::Query::insert()
            .into_table(BankAccounts::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(
                sea_query::OnConflict::column(BankAccounts::AccountId)
                    .update_column(BankAccounts::Frozen)
                    .value(
                        BankAccounts::Amount,
                        sea_query::Expr::col(BankAccounts::Amount)
                            .sub(sea_query::Expr::value(amount)),
                    )
                    .to_owned(),
            )
            .build(sea_query::SqliteQueryBuilder);

        let columns = vec![
            BankAccounts::AccountId,
            BankAccounts::Amount,
            BankAccounts::Frozen,
        ];
        let values: Vec<sea_query::SimpleExpr> = vec![
            to.account_id.get().into(),
            to.amount.into(),
            to.frozen.into(),
        ];

        let to_query = sea_query::Query::insert()
            .into_table(BankAccounts::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(
                sea_query::OnConflict::column(BankAccounts::AccountId)
                    .update_column(BankAccounts::Frozen)
                    .value(
                        BankAccounts::Amount,
                        sea_query::Expr::col(BankAccounts::Amount)
                            .add(sea_query::Expr::value(amount)),
                    )
                    .to_owned(),
            )
            .build(sea_query::SqliteQueryBuilder);

        (from_query, to_query)
    };

    let from_turso_params: Vec<turso::Value> = from_query_values
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

    let to_turso_params: Vec<turso::Value> = to_query_values
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

    Ok(vec![
        DbInstruction::Execute {
            execute: Cow::Owned(from_sql),
            params: from_turso_params,
        },
        DbInstruction::Execute {
            execute: Cow::Owned(to_sql),
            params: to_turso_params,
        },
    ])
}
