pub mod commands;
pub mod db;
pub mod helpers;

mod message;

use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use tokio::{signal, sync::watch};
use tracing::instrument;
use tracing_subscriber::EnvFilter;
use twilight_gateway::{
    CloseFrame, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client;
use twilight_model::{
    application::interaction::InteractionData,
    id::{
        Id,
        marker::{ApplicationMarker, ChannelMarker, UserMarker},
    },
};

use crate::{
    commands::get_commands,
    helpers::sync_configs::{get_receiver, set_watch},
};

#[derive(Deserialize, Clone, Default, Debug)]
pub struct Configs {
    token: String,
    users_blacklist: Vec<Id<UserMarker>>,
    sql_blacklist: Vec<String>,
    allowed_channels: Vec<Id<ChannelMarker>>,
    #[allow(dead_code)]
    database_url: String,
    stickers: Vec<(String, String)>,
    message_replies: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    client: Arc<Client>,
    configs: Arc<Configs>,
    application_id: Id<ApplicationMarker>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("Pretzel_Lover=debug,info")),
        )
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let configs: Configs = {
        let content = std::fs::read("configs.json")?;
        serde_json::from_slice(&content)?
    };
    let intents = Intents::MESSAGE_CONTENT | Intents::GUILD_MESSAGES;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let client = Arc::new(Client::new(configs.token.clone()));
    let shard = Shard::new(ShardId::ONE, configs.token.clone(), intents);

    let application_id = {
        let response = client.current_user_application().await?;

        response.model().await?.id
    };
    client
        .interaction(application_id)
        .set_global_commands(&get_commands(&configs)?)
        .await?;

    //establish_database(&configs.database_url).await?;

    set_watch(Arc::new(AppState {
        client: Arc::clone(&client),
        configs: Arc::new(configs),
        application_id: application_id,
    }))?;

    let state_rx = get_receiver().ok_or(anyhow::anyhow!("State watch wasn't initiated"))?;

    let task = tokio::spawn(dispatcher(shard, shutdown_rx.clone(), state_rx));

    signal::ctrl_c().await?;
    _ = shutdown_tx.send(true);
    task.await?;

    Ok(())
}

#[instrument(fields(shard = %shard.id()), skip_all)]
async fn dispatcher(
    mut shard: Shard,
    mut shutdown: watch::Receiver<bool>,
    state_rx: watch::Receiver<Arc<AppState>>,
) {
    loop {
        let state = state_rx.borrow().clone();

        tokio::select! {
            _ = shutdown.changed() => shard.close(CloseFrame::NORMAL),
            Some(item) = shard.next_event(EventTypeFlags::INTERACTION_CREATE | EventTypeFlags::MESSAGE_CREATE) => {
                let event = match item {
                    Ok(event) => event,
                    Err(source) => {
                        tracing::warn!(?source, "error receiving an event");
                        continue;
                    }
                };

                match event {
                    Event::GatewayClose(_) if *shutdown.borrow() => break,
                    Event::InteractionCreate(e) => {
                        match e.data.as_ref() {
                            Some(InteractionData::ApplicationCommand(data)) => {
                                tokio::spawn(commands::cmd_handler(state.clone(), e.clone(), data.clone()));
                            },
                            Some(InteractionData::ModalSubmit(data)) => {
                                tokio::spawn(commands::modal_handler(state.clone(), e.clone(), data.clone()));
                            },
                            Some(InteractionData::MessageComponent(_)) => { todo!() },
                            Some(invalid) => {
                                tracing::warn!(?invalid, "Unrecognized API");
                                continue;
                            },
                            None => {
                                tracing::warn!("Unrecognized None API");
                                continue;
                            }
                        }
                    },
                    Event::MessageCreate(e) => {
                        tokio::spawn(message::msg_handler(state.clone(), e.clone()));
                    }
                    _ => {}
                }
            }
        }
    }
}
