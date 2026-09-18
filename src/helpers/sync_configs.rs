use std::sync::Arc;

use tokio::sync::{
    OnceCell,
    watch::{self, Receiver, Sender},
};

use crate::AppState;

static RECEIVER: OnceCell<Receiver<Arc<AppState>>> = OnceCell::const_new();
static SENDER: OnceCell<Sender<Arc<AppState>>> = OnceCell::const_new();

pub fn get_receiver() -> Option<Receiver<Arc<AppState>>> {
    RECEIVER.get().cloned()
}

pub fn set_watch(appstate: Arc<AppState>) -> anyhow::Result<()> {
    let (sender, receiver) = watch::channel(appstate);
    RECEIVER.set(receiver)?;
    SENDER.set(sender)?;

    Ok(())
}

pub fn update_configs(state_updated: Arc<AppState>) -> anyhow::Result<()> {
    SENDER
        .get()
        .ok_or(anyhow::anyhow!("Sender wasn't initialized"))?
        .send(state_updated)?;

    Ok(())
}
