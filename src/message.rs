use twilight_model::{gateway::payload::incoming::MessageCreate, id::Id};

use crate::AppState;

#[tracing::instrument(fields(user = ?event.author.id), skip_all, err)]
pub async fn msg_handler(state: AppState, event: Box<MessageCreate>) -> anyhow::Result<()> {
    if event.author.bot || event.content.is_empty() {
        return Ok(());
    }

    if event.channel_id == Id::new(1440695808721686703) {
        // news channel id
        state
            .client
            .crosspost_message(Id::new(1440695808721686703), event.id)
            .await?;
        return Ok(());
    }

    let lower = event.content.to_lowercase();
    tracing::debug!(?lower);
    match () {
        _ if lower.contains("ertone") => {
            state
                .client
                .create_message(event.channel_id)
                .reply(event.id)
                .content("https://cdn.discordapp.com/attachments/1459124234021376000/1544909288604962896/makesweet-o93b8o.gif?ex=6aa4c465&is=6aa372e5&hm=0b151af29453ed3749bb7b9dd6ca0ec6cbc552b7f0d672d6dd34a9a87fae43a5&")
                .await?;
        }
        _ if lower.contains("acharlys") => {
            state
                .client
                .create_message(event.channel_id)
                .reply(event.id)
                .content("https://cdn.discordapp.com/attachments/1459124234021376000/1534850447725035600/makesweet-u3nua9.gif?ex=6aa5165e&is=6aa3c4de&hm=d33337cf79266e2eb296c16d80aac5744c4b86c5c0311763df22405271b36d24&")
                .await?;
        }
        _ => {}
    }

    Ok(())
}
