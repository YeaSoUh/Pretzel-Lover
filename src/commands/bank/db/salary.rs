use std::{sync::Arc, time::Duration};

use tracing::instrument;
use twilight_model::channel::message::MessageFlags;

use crate::{AppState, commands::bank::db::{
    connection::{
        edit_salary_instruction, get_bank_account, get_coming_salary, remove_salary_instruction,
        transfer_pretzels_instruction,
    }, internal_types::{BankAccount, ComponentV2Logger, DbTransaction, Salary, SalaryState::Wait},
}};

#[instrument(err)]
pub async fn establish_salaries(state: Arc<AppState>) -> anyhow::Result<()> {
    loop {
        let upcoming_salary_option;
        if let Ok(upcoming_salary_ok) = get_coming_salary().await {
            upcoming_salary_option = upcoming_salary_ok;
        } else {
            continue;
        }

        match upcoming_salary_option {
            Some(upcoming_salary) => {
                if let Wait(duration) = upcoming_salary.state() {
                    tokio::time::sleep(duration).await;
                }
                let from_account;
                if let Some(bank_account) = get_bank_account(upcoming_salary.from_account).await? {
                    from_account = bank_account;
                } else {
                    tracing::error!("Bank account sending salary doesn't exist;");
                    DbTransaction::new(vec![remove_salary_instruction(upcoming_salary.id).await?])
                        .run()
                        .await?;
                    continue;
                }

                let to_account;
                if let Some(bank_account) = get_bank_account(upcoming_salary.to_account).await? {
                    to_account = bank_account;
                } else {
                    tracing::error!("Bank account receiving salary doesn't exist");
                    DbTransaction::new(vec![remove_salary_instruction(upcoming_salary.id).await?])
                        .run()
                        .await?;
                    continue;
                }

                if upcoming_salary.amount > from_account.amount
                    || from_account.frozen
                    || to_account.frozen
                {
                    DbTransaction::new(vec![remove_salary_instruction(upcoming_salary.id).await?])
                        .run()
                        .await?;
                    continue;
                }
                let result = pay_salary(Arc::clone(&state), &upcoming_salary, &from_account, &to_account).await;
                if result.is_err() {
                    tokio::time::sleep(Duration::from_secs(10)).await
                }
            }
            None => tokio::time::sleep(Duration::from_mins(5)).await,
        }
    }
}

async fn pay_salary(
    state: Arc<AppState>,
    salary: &Salary,
    from_account: &BankAccount,
    to_account: &BankAccount,
) -> anyhow::Result<()> {
    let new_run = salary.run_at + chrono::Duration::hours(salary.interval_hours);

    let new_salary = Salary {
        id: salary.id,
        from_account: salary.from_account,
        to_account: salary.to_account,
        amount: salary.amount,
        interval_hours: salary.interval_hours,
        run_at: new_run,
    };

    let mut instructions = vec![edit_salary_instruction(&new_salary).await?];
    instructions.append(
        &mut transfer_pretzels_instruction(&from_account, &to_account, salary.amount).await?,
    );
    DbTransaction::new(instructions).run().await?;

    state
        .client
        .create_message(state.configs.logging_channel)
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(&[salary.to_component()])
        .await?;

    Ok(())
}
