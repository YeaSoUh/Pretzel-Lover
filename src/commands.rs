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
    command::{BooleanBuilder, ChannelBuilder, CommandBuilder, StringBuilder},
};

use crate::{AppState, Configs, commands};

pub mod edit;
pub mod get;
pub mod morgoft;
pub mod remove;
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

pub fn get_commands(configs: &Configs) -> anyhow::Result<Vec<Command>> {
    Ok(vec![
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
        CommandBuilder::new("say", "Say as a bot", CommandType::ChatInput)
            .option(ChannelBuilder::new("channel", "Pick a channel to send"))
            .option(
                StringBuilder::new(
                    "Sticker",
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
        CommandBuilder::new("Sentence to Morgoft", "idk", CommandType::User)
            .contexts(vec![InteractionContextType::Guild])
            .validate()?
            .build(),
    ])
}

pub async fn cmd_handler(
    state: AppState,
    event: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let result = match data.name.as_str() {
        "edit_db" => commands::edit::run(state, &event).await,
        "get_db" => commands::get::run(state, &event).await,
        "remove_db" => commands::remove::run(state, &event).await,
        "search_db" => commands::search::run(state, &event).await,
        "say" => Err(anyhow::anyhow!("TODO command")),
        "edit" => Err(anyhow::anyhow!("TODO command")),
        "Sentence to Morgoft" => commands::morgoft::run(state, &event).await,
        // they are discontinued due to low usage but they can come back
        // "channel_manager" => todo!(),
        // "kitty" => todo!(),
        // "doge" => todo!(),
        _ => unreachable!("Non existent command"),
    };

    if let Err(e) = &result {
        println!("AN ERROR!!!!!!!!!!! HERE:\n{e}")
    }
    result
}
