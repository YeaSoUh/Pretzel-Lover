use chrono::{DateTime, Utc};
use serde::Deserialize;
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

#[derive(Deserialize, Debug)]
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

pub enum PaymentState {
    Due,
    Wait(DateTime<Utc>), // ISO 8601 datetime
}

pub struct Payment {
    pub id: [u8; 16], // BLOB (raw binary data) of uuid v7
    pub from: Id<UserMarker>,
    pub to: Id<UserMarker>,
    pub amount: i64,
    pub run_at: DateTime<Utc>,
}

impl Payment {
    pub fn state(&self) -> PaymentState {
        if self.run_at < Utc::now() {
            PaymentState::Due
        } else {
            PaymentState::Wait(self.run_at)
        }
    }
}

impl TryFrom<&Row> for Payment {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut idx = 0;
        let mut next = || {
            let i = idx;
            idx += 1;
            i
        };

        Ok(Payment {
            id: row.get(next())?,
            from: Id::new_checked((*row.get_value(next())?.as_integer().ok_or(anyhow::anyhow!("User sending a payment isn't a valid id"))?).try_into()?).ok_or(anyhow::anyhow!("User sending a payment has a zero id"))?,
            to: Id::new_checked((*row.get_value(next())?.as_integer().ok_or(anyhow::anyhow!("User receiving a payment isn't a valid id"))?).try_into()?).ok_or(anyhow::anyhow!("User receving a payment has a zero id"))?,
            amount: row.get(next())?,
            run_at: DateTime::parse_from_rfc3339(row.get_value(next())?.as_text().ok_or(anyhow::anyhow!("Invalid ISO 8601 datetime format"))?)?.with_timezone(&Utc),
        })
    }
}

impl TryFrom<Row> for Payment {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Payment::try_from(&row)
    }
}