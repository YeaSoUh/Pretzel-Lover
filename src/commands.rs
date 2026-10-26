use std::sync::Arc;

use twilight_http::{Response, response::marker::EmptyBody};
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::{
            InteractionContextType, application_command::CommandData, modal::ModalInteractionData,
        },
    },
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    oauth::ApplicationIntegrationType,
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    command::{
        BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder, StringBuilder, UserBuilder,
    },
};

use crate::{AppState, Configs, commands};

pub mod add_role_employee;
pub mod bank;
//pub mod edit;
//pub mod get;
pub mod pretzel;
//pub mod remove;
//pub mod search;
pub mod update_configs;

pub async fn defer(
    state: Arc<AppState>,
    event: &InteractionCreate,
    ephemeral: bool,
) -> Result<Response<EmptyBody>, twilight_http::Error> {
    let ephemeral = {
        if ephemeral {
            Some(MessageFlags::EPHEMERAL)
        } else {
            None
        }
    };

    state
        .client
        .interaction(state.application_id)
        .create_response(
            event.id,
            &event.token,
            &twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
                data: {
                    if let Some(flags) = ephemeral {
                        Some(InteractionResponseDataBuilder::new().flags(flags).build())
                    } else { None }
                },
            },
        )
        .await
}

pub fn response(data: InteractionResponseData) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(data),
    }
}

pub async fn respond(state: Arc<AppState>, token: &str, content: &str) -> anyhow::Result<()> {
    state
        .client
        .interaction(state.application_id)
        .create_followup(&token)
        .content(content)
        .await?;

    Ok(())
}

pub fn get_commands(configs: &Configs) -> anyhow::Result<Vec<Command>> {
    Ok(vec![
        /*
        CommandBuilder::new("edit_db", "Edit a planet/moon", CommandType::ChatInput)
            .option(StringBuilder::new("index", "What planet/moon to edit").required(true))
            .option(StringBuilder::new("input", "Self explanatory").required(true))
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
        CommandBuilder::new("get_db", "Get a planet/moon", CommandType::ChatInput)
            .option(StringBuilder::new("index", "What planet/moon to get").required(true))
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
        CommandBuilder::new(
            "search_db",
            "Search for planets/moons",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("input", "Self explanatory").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new("remove_db", "Remove a planet/moon", CommandType::ChatInput)
            .option(
                StringBuilder::new("index", "An index of a planet/moon to remove").required(true),
            )
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
        */
        CommandBuilder::new("say", "Say as a bot", CommandType::ChatInput)
            .option(ChannelBuilder::new("channel", "Pick a channel to send"))
            .option(
                StringBuilder::new(
                    "sticker",
                    "Type a sticker's id to send a message with sticker",
                )
                .choices(configs.stickers.clone()),
            )
            .option(StringBuilder::new(
                "reply",
                "Type a message's url to reply (will overwrite channel parameter)",
            ))
            .option(StringBuilder::new(
                "forward",
                "Paste a message's url to forward it",
            ))
            .option(BooleanBuilder::new(
                "mention_author",
                "Mention author while replying?",
            ))
            .option(BooleanBuilder::new(
                "silent",
                "Should the message be silent?",
            ))
            .option(BooleanBuilder::new(
                "tts",
                "Should Discord say the message's content?",
            ))
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
        CommandBuilder::new("edit", "Edits a message by a bot", CommandType::ChatInput)
            .option(
                StringBuilder::new("message_url", "Insert a message link to edit").required(true),
            )
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
        CommandBuilder::new("Sentence to Morgoft", "", CommandType::User)
            .contexts(vec![InteractionContextType::Guild])
            .integration_types(vec![ApplicationIntegrationType::GuildInstall])
            .validate()?
            .build(),
        CommandBuilder::new("Add Employee Role", "", CommandType::User)
            .contexts(vec![InteractionContextType::Guild])
            .integration_types(vec![ApplicationIntegrationType::GuildInstall])
            .validate()?
            .build(),
        CommandBuilder::new(
            "update_configs",
            "Updates configs of a bot (only owner is allowed to run this)",
            CommandType::ChatInput,
        )
        .contexts(vec![InteractionContextType::Guild])
        .integration_types(vec![ApplicationIntegrationType::GuildInstall])
        .validate()?
        .build(),
        CommandBuilder::new(
            "add_magells",
            "Adds magells to a bank account (only admins since it can inflate the economy)",
            CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "Who to pay").required(true))
        .option(IntegerBuilder::new("amount", "Amount to transfer").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "remove_magells",
            "Removes magells from a bank account (only admins since it can inflate the economy)",
            CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "User to remove magells from").required(true))
        .option(IntegerBuilder::new("amount", "Amount to remove").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "freeze_bank_account",
            "Freezes (and unfreezes) user's bank account",
            CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "User to freeze").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "register_bank_account",
            "Registers a new bank account (one account per user)",
            CommandType::ChatInput,
        )
        .contexts(vec![InteractionContextType::Guild])
        .integration_types(vec![ApplicationIntegrationType::GuildInstall])
        .validate()?
        .build(),
        CommandBuilder::new(
            "delete_bank_account",
            "Delete your bank account",
            CommandType::ChatInput,
        )
        .contexts(vec![InteractionContextType::Guild])
        .integration_types(vec![ApplicationIntegrationType::GuildInstall])
        .validate()?
        .build(),
        CommandBuilder::new(
            "transfer",
            "Transfer money between users",
            CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "Who to pay").required(true))
        .option(IntegerBuilder::new("amount", "Amount to transfer").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "salary",
            "Automatically pay someone on a schedule",
            CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "Who to pay").required(true))
        .option(IntegerBuilder::new("amount", "Amount to transfer").required(true))
        .option(IntegerBuilder::new("interval_hours", "How often to pay").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "balance",
            "Check balance of the user (or yours)",
            CommandType::ChatInput,
        )
        .option(
            UserBuilder::new(
                "user",
                "User to check balance of (if left empty will display your balance)",
            )
            .required(false),
        )
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
        CommandBuilder::new(
            "user_bank_information",
            "Check user's bank information (or yours)",
            CommandType::ChatInput,
        )
        .option(
            UserBuilder::new(
                "user",
                "User to check bank information (if left empty will display your information)",
            )
            .required(false),
        )
        .contexts(vec![InteractionContextType::Guild])
        .validate()?
        .build(),
    ])
}

#[tracing::instrument(fields(user = ?event.author_id()), skip_all, err)]
pub async fn cmd_handler(
    state: Arc<AppState>,
    event: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let result = match data.as_ref().name.as_str() {
        // they are discontinued due to low usage but they can come back
        //"edit_db" => commands::edit::run(state, &event, &data).await,
        //"get_db" => commands::get::run(state, &event, &data).await,
        //"remove_db" => commands::remove::run(state, &event, &data).await,
        //"search_db" => commands::search::run(state, &event, &data).await,
        "say" => commands::pretzel::say::run(state, &event, &data).await,
        "edit" => commands::pretzel::edit::run(state, &event, &data).await,
        "Sentence to Morgoft" => commands::pretzel::morgoft::run(state, &event, &data).await,
        "Add Employee Role" => commands::add_role_employee::run(state, &event, &data).await,
        "update_configs" => commands::update_configs::run(state, &event, &data).await,
        "add_magells" => commands::bank::add_pretzels::run(state, &event, &data).await,
        "remove_magells" => commands::bank::remove_pretzels::run(state, &event, &data).await,
        "freeze_bank_account" => commands::bank::freeze::run(state, &event, &data).await,
        "register_bank_account" => commands::bank::register::run(state, &event, &data).await,
        "delete_bank_account" => commands::bank::delete_account::run(state, &event, &data).await,
        "transfer" => commands::bank::transfer::run(state, &event, &data).await,
        "salary" => commands::bank::salary::run(state, &event, &data).await,
        // discontinued too
        // "channel_manager" => todo!(),
        // "kitty" => todo!(),
        // "doge" => todo!(),
        _ => unreachable!("Non existent command"),
    };

    if let Err(e) = &result {
        tracing::error!(?e, "AN ERROR!!!!!!!!!!!")
    }
    result
}

#[tracing::instrument(fields(user = ?event.author_id()), skip_all, err)]
pub async fn modal_handler(
    state: Arc<AppState>,
    event: Box<InteractionCreate>,
    data: Box<ModalInteractionData>,
) -> anyhow::Result<()> {
    let result = match data.as_ref().custom_id.as_str() {
        id if id.starts_with("say_modal") => {
            commands::pretzel::say::modal(state, &event, &data).await
        }
        id if id.starts_with("edit_modal") => {
            commands::pretzel::edit::modal(state, &event, &data).await
        }
        _ => unreachable!("Non existent command"),
    };

    if let Err(e) = &result {
        tracing::error!(?e, "AN ERROR!!!!!!!!!!!")
    }
    result
}
