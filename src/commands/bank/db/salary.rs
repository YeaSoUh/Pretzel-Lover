use std::time::Duration;

use tracing::instrument;

use crate::commands::bank::db::{
    connection::{
        edit_bank_account, edit_salary, get_bank_account, get_coming_salary, remove_salary,
    },
    internal_types::{
        BankAccount, Salary,
        SalaryState::{Due, Wait},
    },
};

#[instrument(err)]
pub async fn establish_salaries() -> anyhow::Result<()> {
    loop {
        let upcoming_salary_option;
        if let Ok(upcoming_salary_ok) = get_coming_salary().await {
            upcoming_salary_option = upcoming_salary_ok;
        } else {
            continue;
        }

        match upcoming_salary_option {
            Some(upcoming_salary) => {
                let from_account;
                if let Some(bank_account) = get_bank_account(upcoming_salary.from).await? {
                    from_account = bank_account;
                } else {
                    tracing::error!(
                        "Bank account sending salary doesn't exist; Deleted the salary just in case"
                    );
                    remove_salary(uuid::Uuid::from_bytes(upcoming_salary.id)).await?;
                    continue;
                }

                let to_account;
                if let Some(bank_account) = get_bank_account(upcoming_salary.to).await? {
                    to_account = bank_account;
                } else {
                    tracing::error!(
                        "Bank account receiving salary doesn't exist; Deleted the salary just in case"
                    );
                    remove_salary(uuid::Uuid::from_bytes(upcoming_salary.id)).await?;
                    continue;
                }

                if upcoming_salary.amount > from_account.amount
                    || from_account.frozen
                    || to_account.frozen
                {
                    remove_salary(uuid::Uuid::from_bytes(upcoming_salary.id)).await?;
                    continue;
                }

                match upcoming_salary.state() {
                    Due => pay_salary(&upcoming_salary, &from_account, &to_account).await?,
                    Wait(duration) => {
                        tokio::time::sleep(duration).await;
                        pay_salary(&upcoming_salary, &from_account, &to_account).await?;
                    }
                }
            }
            None => tokio::time::sleep(Duration::from_mins(5)).await,
        }
    }
}

async fn pay_salary(
    salary: &Salary,
    from_account: &BankAccount,
    to_account: &BankAccount,
) -> anyhow::Result<()> {
    const MAX_INTERVAL_HOURS: u64 = 24 * 365; // 1 year

    let hours = salary.interval_hours.clamp(1, MAX_INTERVAL_HOURS) as i64;
    let new_run = salary.run_at + chrono::Duration::hours(hours);

    let new_salary = Salary {
        id: salary.id,
        from: salary.from,
        to: salary.to,
        amount: salary.amount,
        interval_hours: salary.interval_hours,
        run_at: new_run,
    };

    edit_salary(&new_salary).await?;

    let new_from_account = &BankAccount {
        account_id: from_account.account_id,
        amount: from_account.amount - salary.amount,
        frozen: from_account.frozen,
    };
    let new_to_account = &BankAccount {
        account_id: to_account.account_id,
        amount: to_account.amount + salary.amount,
        frozen: to_account.frozen,
    };

    edit_bank_account(new_from_account).await?;
    edit_bank_account(new_to_account).await?;

    Ok(())
}
