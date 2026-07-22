pub mod commands;
pub mod db;

use std::sync::Arc;

use serde::Deserialize;
use tokio::{signal, sync::watch};
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

use crate::{commands::get_commands, db::connection::establish_database};

#[derive(Deserialize, Clone)]
pub struct Configs {
    token: String,
    users_blacklist: Vec<Id<UserMarker>>,
    sql_blacklist: Vec<String>,
    allowed_channels: Vec<Id<ChannelMarker>>,
    database_url: String,
}

#[derive(Clone)]
pub struct AppState {
    client: Arc<Client>,
    configs: Arc<Configs>, // already includes db
    application_id: Id<ApplicationMarker>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let configs: Configs = {
        let content = std::fs::read("configs.json")?;
        serde_json::from_slice(&content)?
    };
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let client = Arc::new(Client::new(configs.token.clone()));
    let shard = Shard::new(ShardId::ONE, configs.token.clone(), Intents::empty());

    let application_id = {
        let response = client.current_user_application().await?;

        response.model().await?.id
    };
    client
        .interaction(application_id)
        .set_global_commands(&get_commands())
        .await?;

    establish_database(&configs.database_url).await?;

    let task = tokio::spawn(dispatcher(
        AppState {
            client: Arc::clone(&client),
            configs: Arc::new(configs),
            application_id: application_id,
        },
        shard,
        shutdown_rx.clone(),
    ));

    signal::ctrl_c().await?;
    _ = shutdown_tx.send(true);
    task.await?;

    Ok(())
}

async fn dispatcher(state: AppState, mut shard: Shard, mut shutdown: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => shard.close(CloseFrame::NORMAL),
            Some(item) = shard.next_event(EventTypeFlags::INTERACTION_CREATE) => {
                let event = match item {
                    Ok(event) => event,
                    Err(source) => {
                        eprintln!("error receiving event {source:?}");
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
                            Some(InteractionData::ModalSubmit(_)) => { todo!() },
                            Some(InteractionData::MessageComponent(_)) => { todo!() },
                            Some(_) => { unreachable!("Every type is already covered; Discord API fault") },
                            None => { unreachable!() }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
