use twilight_http::{Response, response::marker::EmptyBody};
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::{InteractionContextType, application_command::CommandData},
    },
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    command::{CommandBuilder, StringBuilder},
};

use crate::{AppState, commands};

pub mod edit;
pub mod get;
pub mod search;

pub async fn defer(
    state: AppState,
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

pub fn get_commands() -> Vec<Command> {
    vec![
        CommandBuilder::new("edit_db", "Edit a planet/moon", CommandType::ChatInput)
            .option(StringBuilder::new("index", "What planet/moon to edit").required(true))
            .option(StringBuilder::new("input", "Self explanatory").required(true))
            .contexts(vec![InteractionContextType::Guild])
            .build(),
        CommandBuilder::new("get_db", "Get a planet/moon", CommandType::ChatInput)
            .option(StringBuilder::new("index", "What planet/moon to get").required(true))
            .contexts(vec![InteractionContextType::Guild])
            .build(),
        CommandBuilder::new(
            "search_db",
            "Search for planets/moons",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("input", "Self explanatory").required(true))
        .contexts(vec![InteractionContextType::Guild])
        .build(),
    ]
}

pub async fn cmd_handler(
    state: AppState,
    event: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    match data.name.as_str() {
        "edit_db" => commands::edit::run(state, &event).await?,
        "get_db" => commands::get::run(state, &event).await?,
        "search_db" => commands::search::run(state, &event).await?,
        "say" => todo!(),
        "edit" => todo!(),
        "channel_manager" => todo!(),
        "kitty" => todo!(),
        "doge" => todo!(),
        _ => unreachable!("Non existent command"),
    }
    Ok(())
}
