use std::time::Duration;

use chrono::{DateTime, Utc};
use turso::Row;
use twilight_model::{
    channel::message::Component,
    id::{Id, marker::UserMarker},
};
use twilight_util::builder::message::{ContainerBuilder, SeparatorBuilder, TextDisplayBuilder};
use uuid::Uuid;

use crate::{
    commands::bank::db::connection::{get_bank_account, get_db, transfer_pretzels_instruction},
    db::{
        helpers::execute,
        types::{DbInstruction, DbResults},
    },
};

pub trait ComponentV2Logger {
    fn to_component(&self) -> Component;
}

pub struct DbTransaction {
    instructions: Vec<DbInstruction>,
}

impl DbTransaction {
    pub fn new(instructions: Vec<DbInstruction>) -> DbTransaction {
        DbTransaction {
            instructions: instructions,
        }
    }

    pub async fn run(&self) -> anyhow::Result<DbResults> {
        let db = get_db()?;

        Ok(execute(db, self.instructions.as_slice()).await?)
    }
}

#[derive(sea_query::Iden)]
pub(in crate::commands::bank::db) enum BankAccounts {
    Table,
    AccountId,
    Amount,
    Frozen,
}

#[derive(Debug)]
pub struct BankAccount {
    pub account_id: Id<UserMarker>,
    pub amount: i64,
    pub frozen: bool,
}

impl ComponentV2Logger for BankAccount {
    fn to_component(&self) -> Component {
        Component::Container(
            ContainerBuilder::new()
                .accent_color(Some(0x216FE7)) // 2191335
                .spoiler(false)
                .component(TextDisplayBuilder::new("# BANK ACCOUNT INFORMATION").build())
                .component(SeparatorBuilder::new().build())
                .component(
                    TextDisplayBuilder::new(format!("Account ID: {}", self.account_id)).build(),
                )
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("Amount: {}", self.amount)).build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("Frozen: {}", self.frozen)).build())
                .build(),
        )
    }
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
            account_id: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or_else(|| anyhow::anyhow!("Bank user isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or_else(|| anyhow::anyhow!("Bank user has a zero id"))?,
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
    FromAccount,
    ToAccount,
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
    pub id: Uuid,
    pub from_account: Id<UserMarker>,
    pub to_account: Id<UserMarker>,
    pub amount: i64,
    pub interval_hours: i64,
    pub run_at: DateTime<Utc>,
}

impl Salary {
    pub fn state(&self) -> SalaryState {
        (self.run_at - Utc::now())
            .to_std()
            .map_or(SalaryState::Due, SalaryState::Wait)
    }
}

impl ComponentV2Logger for Salary {
    fn to_component(&self) -> Component {
        Component::Container(
            ContainerBuilder::new()
                .accent_color(Some(0x216FE7)) // 2191335
                .spoiler(false)
                .component(TextDisplayBuilder::new("# SALARY PROCESS").build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("From: {}", self.from_account)).build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("To: {}", self.to_account)).build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("Amount: {}", self.amount)).build())
                .component(SeparatorBuilder::new().build())
                .component(
                    TextDisplayBuilder::new(format!("Every: {} hour(s)", self.interval_hours))
                        .build(),
                )
                .component(SeparatorBuilder::new().build())
                .component(
                    TextDisplayBuilder::new(format!(
                        "Next salary at: <:t{}:R>",
                        self.run_at.timestamp()
                    ))
                    .build(),
                )
                .build(),
        )
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

        print(row.get(5));

        Ok(Salary {
            id: Uuid::parse_str(&row.get::<String>(next())?)?,
            from_account: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or(anyhow::anyhow!("User sending salary isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or_else(|| anyhow::anyhow!("User sending salary has a zero id"))?,
            to_account: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or_else(|| anyhow::anyhow!("User receiving salary isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or_else(|| anyhow::anyhow!("User receving salary has a zero id"))?,
            amount: row.get(next())?,
            interval_hours: row.get(next())?,
            run_at: DateTime::parse_from_rfc3339(
                row.get_value(next())?
                    .as_text()
                    .ok_or_else(|| anyhow::anyhow!("Invalid ISO 8601 datetime format"))?,
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

pub struct Payment {
    pub from_account: Id<UserMarker>,
    pub to_account: Id<UserMarker>,
    pub amount: i64,
    pub reason: Option<String>,
}

impl Payment {
    // aka pay
    pub async fn process_payment(&self) -> anyhow::Result<()> {
        let db = get_db()?;

        let from_account;
        if let Some(bank_account) = get_bank_account(self.from_account).await? {
            from_account = bank_account;
        } else {
            return Err(anyhow::anyhow!(
                "Bank account sending the payment doesn't exist"
            ));
        }

        let to_account;
        if let Some(bank_account) = get_bank_account(self.to_account).await? {
            to_account = bank_account;
        } else {
            return Err(anyhow::anyhow!(
                "Bank account receiving the payment doesn't exist"
            ));
        }

        if from_account.amount - self.amount < 0 {
            anyhow::bail!("Not enough money to transfer");
        }
        if from_account.frozen || to_account.frozen {
            anyhow::bail!("Frozen account")
        }

        execute(
            db,
            transfer_pretzels_instruction(&from_account, &to_account, self.amount)
                .await?
                .as_slice(),
        )
        .await?;

        Ok(())
    }
}

impl ComponentV2Logger for Payment {
    fn to_component(&self) -> Component {
        Component::Container(
            ContainerBuilder::new()
                .accent_color(Some(0x216FE7)) // 2191335
                .spoiler(false)
                .component(TextDisplayBuilder::new("# PAYMENT PROCESS").build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("From: {}", self.from_account)).build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("To: {}", self.to_account)).build())
                .component(SeparatorBuilder::new().build())
                .component(TextDisplayBuilder::new(format!("Amount: {}", self.amount)).build())
                .component(SeparatorBuilder::new().build())
                .component(
                    TextDisplayBuilder::new(format!(
                        "Reason: {}",
                        self.reason.as_deref().unwrap_or("No reason provided")
                    ))
                    .build(),
                )
                .build(),
        )
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
            from_account: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or_else(|| anyhow::anyhow!("User sending a payment isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or_else(|| anyhow::anyhow!("User sending a payment has a zero id"))?,
            to_account: Id::new_checked(
                (*row
                    .get_value(next())?
                    .as_integer()
                    .ok_or_else(|| anyhow::anyhow!("User receiving a payment isn't a valid id"))?)
                .try_into()?,
            )
            .ok_or_else(|| anyhow::anyhow!("User receving a payment has a zero id"))?,
            amount: row.get(next())?,
            reason: row.get(next())?,
        })
    }
}

impl TryFrom<Row> for Payment {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Payment::try_from(&row)
    }
}
