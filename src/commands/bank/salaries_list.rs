use std::sync::Arc;

use chrono::{DateTime, Utc};
use tracing::instrument;
use twilight_model::{
    application::interaction::{
        Interaction, application_command::CommandData, message_component::MessageComponentInteractionData,
    }, channel::message::{
        Component, EmojiReactionType, MessageFlags,
        component::{ButtonStyle, Container, SeparatorSpacingSize},
    }, id::{Id, marker::UserMarker},
};
use twilight_util::builder::message::{
    ActionRowBuilder, ButtonBuilder, ContainerBuilder, SeparatorBuilder, TextDisplayBuilder,
};
use uuid::Uuid;

use crate::{
    AppState,
    commands::{
        bank::db::{
            connection::{get_bank_account, search_salaries},
            internal_types::Salary,
        },
        defer, respond,
    },
};

#[instrument(skip_all, err)]
pub async fn run(
    state: Arc<AppState>,
    event: Interaction,
    _data: CommandData,
) -> anyhow::Result<()> {
    defer(state.clone(), &event, true).await?;

    let user_id = event
        .author_id()
        .ok_or_else(|| anyhow::anyhow!("No author id"))?;

    let bank_account = get_bank_account(user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("The bank account doesn't exist"))?;

    if bank_account.frozen {
        respond(
            state.clone(),
            &event.token,
            "You are not allowed to delete a bank account",
        )
        .await?;
        return Ok(());
    }

    let page = 1;
    let salaries = match search_salaries(&bank_account, page).await? {
        Some(salaries) => salaries,
        None => {
            respond(state.clone(), &event.token, "No salaries yet").await?;
            return Ok(());
        }
    };
    let expiration_date = Utc::now() + chrono::Duration::minutes(5);

    state
        .client
        .interaction(state.application_id)
        .create_followup(&event.token)
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(&[salaries_list_container(
            page.into(),
            salaries,
            false,
            false,
            expiration_date,
            &user_id,
        )
        .into()])
        .await?;

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn previous_page(
    state: Arc<AppState>,
    event: Interaction,
    data: MessageComponentInteractionData,
) -> anyhow::Result<()> {
    let container = if let Component::Container(container) = event
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No message in button component"))?
        .components
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("No components"))?
    {
        container
    } else {
        anyhow::bail!("Not a container component")
    };

    let page_string = if let Component::TextDisplay(text) = container
        .components
        .get(2)
        .ok_or_else(|| anyhow::anyhow!("No component in the container"))?
    {
        text.content
            .strip_prefix("Page:")
            .ok_or_else(|| anyhow::anyhow!("No \"Page:\" prefix"))?
            .parse::<u32>()?
    } else {
        anyhow::bail!("no text display component in the container")
    };

    let timestamp = if let Component::TextDisplay(text) = container
        .components
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("No component in the container"))?
    {
        chrono::DateTime::from_timestamp(
            text.content
                .strip_prefix("Expires in <t:")
                .ok_or_else(|| anyhow::anyhow!("No \"Expires in <t:\" prefix"))?
                .strip_suffix(":R>")
                .ok_or_else(|| anyhow::anyhow!("No \":R>\" suffifx"))?
                .parse::<i64>()?,
            0,
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?
    } else {
        anyhow::bail!("no text display component in the container")
    };

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn next_page(
    state: Arc<AppState>,
    event: Interaction,
    _data: MessageComponentInteractionData,
) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn salaries_list_container(
    page: u32,
    salaries: Vec<Salary>,
    previous_page_disabled: bool,
    next_page_disabled: bool,
    expiration_date: DateTime<Utc>,
    owner: &Id<UserMarker>,
) -> Container {
    let mut builder = ContainerBuilder::new()
        .accent_color(Some(2191335))
        .spoiler(false)
        .component(TextDisplayBuilder::new("## Salaries").build())
        .component(
            TextDisplayBuilder::new(format!("Expires in <t:{}:R>", expiration_date.timestamp()))
                .build(),
        )
        .component(TextDisplayBuilder::new(format!("Page {}", page)).build())
        .component(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(SeparatorSpacingSize::Large)
                .build(),
        );

    for (i, salary) in salaries.iter().enumerate() {
        let content = match &salary.from_account {
            id if id.get() == owner.get() => format!(
                "Sending salary to {} every {} hours(s), next payment in <t:{}:R>. Id of the salary: {}",
                salary.to_account,
                salary.interval_hours,
                salary.run_at.timestamp(),
                Uuid::from_bytes(salary.id).to_string()
            ),
            _ => format!(
                "Getting salary from {} every {} hour(s), next payment in {}.",
                salary.from_account,
                salary.interval_hours,
                salary.run_at.timestamp()
            ),
        };

        builder = builder.component(TextDisplayBuilder::new(content).build());
        if i != salaries.len() - 1 {
            builder = builder.component(
                SeparatorBuilder::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Small)
                    .build(),
            )
        }
    }

    builder
        .component(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(SeparatorSpacingSize::Large)
                .build(),
        )
        .component(
            ActionRowBuilder::new()
                .component(
                    ButtonBuilder::new(ButtonStyle::Primary)
                        .label("Previous Page")
                        .emoji(EmojiReactionType::Unicode {
                            name: "⬅️".into()
                        })
                        .disabled(previous_page_disabled)
                        .custom_id("previous_page")
                        .build(),
                )
                .component(
                    ButtonBuilder::new(ButtonStyle::Primary)
                        .label("Next Page")
                        .emoji(EmojiReactionType::Unicode {
                            name: "➡️".into()
                        })
                        .disabled(next_page_disabled)
                        .custom_id("next_page")
                        .build(),
                )
                .build(),
        )
        .build()
}
