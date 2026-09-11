use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use tokio::time::Instant;
use tracing::instrument;

pub trait Expiring {
    fn expires_at(&self) -> Instant;

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at()
    }
}

#[instrument(skip(map), fields(check_interval = ?check_interval))]
pub async fn purge<T>(map: Arc<DashMap<String, Arc<T>>>, check_interval: Duration)
where
    T: Expiring + Send + Sync + 'static
{
    loop {
        tokio::time::sleep(check_interval).await;

        let map_clone = Arc::clone(&map);
        let result = tokio::task::spawn_blocking(move || {
            map_clone.retain(|_, val| {
                !val.is_expired()
            });
        }).await;

        if let Err(e) = result {
            tracing::error!(error = ?e, "An error happened during background purging of a DashMap");
        }
    }
}