use std::time::Duration;

use chrono::{DateTime, Utc};
use turso::Row;
use twilight_model::id::{Id, marker::UserMarker};

#[allow(dead_code)] // for now
#[derive(sea_query::Iden)]
pub(in crate::commands::bank::db) enum BankAccounts {
    Table,
    Id,
    Amount,
    Frozen,
}

#[derive(Debug)]
pub struct BankAccount {
    pub id: String,
    pub amount: u64,
    pub frozen: bool,
}

impl TryFrom<&Row> for BankAccount {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut idx = 0;
        let mut next = || {
            let i = idx;
            idx += 1;
            i
        };

        Ok(BankAccount {
            id: row.get(next())?,
            amount: row.get(next())?,
            frozen: row.get(next())?,
        })
    }
}

impl TryFrom<Row> for BankAccount {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        BankAccount::try_from(&row)
    }
}

#[derive(sea_query::Iden)]
pub(in crate::commands::bank::db) enum Salaries {
    Table,
    Id,
    From,
    To,
    Amount,
    IntervalHours,
    RunAt,
}

#[derive(Debug)]
pub enum SalaryState {
    Due,
    Wait(Duration),
}

#[derive(Debug)]
pub struct Salary {
    pub id: [u8; 16], // BLOB (raw binary data) of uuid v7
    pub from: Id<UserMarker>,
    pub to: Id<UserMarker>,
    pub amount: u64,
    pub interval_hours: u64,
    pub run_at: DateTime<Utc>,
}

impl Salary {
    pub fn state(&self) -> SalaryState {
        (self.run_at - Utc::now()).to_std().map_or(SalaryState::Due, SalaryState::Wait)
    }
}

impl TryFrom<&Row> for Salary {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut idx = 0;
        let mut next = || {
            let i = idx;
            idx += 1;
            i
        };

        Ok(Salary {
            id: row.get(next())?,
            from: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or(anyhow::anyhow!("User sending a payment isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or(anyhow::anyhow!("User sending a payment has a zero id"))?,
            to: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or(anyhow::anyhow!("User receiving a payment isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or(anyhow::anyhow!("User receving a payment has a zero id"))?,
            amount: row.get(next())?,
            interval_hours: row.get(next())?,
            run_at: DateTime::parse_from_rfc3339(
                row.get_value(next())?
                    .as_text()
                    .ok_or(anyhow::anyhow!("Invalid ISO 8601 datetime format"))?,
            )?
            .with_timezone(&Utc),
        })
    }
}

impl TryFrom<Row> for Salary {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Salary::try_from(&row)
    }
}
