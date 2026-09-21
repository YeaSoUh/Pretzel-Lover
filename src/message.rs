use std::sync::Arc;

use twilight_model::{gateway::payload::incoming::MessageCreate, id::Id};

use crate::AppState;

#[tracing::instrument(fields(user = ?event.author.id), skip_all, err)]
pub async fn msg_handler(state: Arc<AppState>, event: Box<MessageCreate>) -> anyhow::Result<()> {
    if event.author.bot || (event.content.is_empty() && event.attachments.is_empty()) {
        return Ok(());
    }

    if event.channel_id == Id::new(1440695808721686703) {
        // news channel id
        state
            .client
            .crosspost_message(event.channel_id, event.id)
            .await?;
        return Ok(());
    }

    if event.channel_id == Id::new(1410924213430259783) // builds channel
        && !event.attachments.is_empty()
    {

        let content = if event.content.is_empty() {
            &event.attachments[0].filename
        } else {
            &event.content
        };
        
        state
            .client
            .create_reaction(
                event.channel_id,
                event.id,
                &twilight_http::request::channel::reaction::RequestReactionType::Custom {
                    id: Id::new(1445413531649310761),
                    name: Some("peepohappy"),
                },
            )
            .await?;

        state
            .client
            .create_reaction(
                event.channel_id,
                event.id,
                &twilight_http::request::channel::reaction::RequestReactionType::Unicode {
                    name: "🗑️",
                },
            )
            .await?;

        state
            .client
            .create_thread_from_message(event.channel_id, event.id, content)
            .await?;

        return Ok(());
    }

    let lower = event.content.to_lowercase();
    if let Some((_, url)) = state
        .configs
        .message_replies
        .iter()
        .find(|(key, _)| lower.contains(key.as_str()))
    {
        state
            .client
            .create_message(event.channel_id)
            .reply(event.id)
            .content(url)
            .await?;
    }

    Ok(())
}
