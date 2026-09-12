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
        _ if lower.contains("urmone") => {
            state
                .client
                .create_message(event.channel_id)
                .reply(event.id)
                .content("https://cdn.discordapp.com/attachments/1410923843710746676/1548287385165504542/makesweet-3pedo8.gif?ex=6aa6827e&is=6aa530fe&hm=a8e35bab4249a6ca88decb26ee547f1451318a75c07f9dbc52c630b7603fa985&")
                .await?;
        }
        _ if lower.contains("kameva") => {
            state
                .client
                .create_message(event.channel_id)
                .reply(event.id)
                .content("https://cdn.discordapp.com/attachments/1420109753878839338/1426289136465346660/makesweet-aaxbs5.gif?ex=68eaaec5&is=68e95d45&hm=bdc97b9bd722cba025a3e71d4701d646438e897b9ab5f1a12c37f3161b8e9674&")
                .await?;
        }
        _ if lower.contains("konami") => {
            state
                .client
                .create_message(event.channel_id)
                .reply(event.id)
                .content("https://cdn.discordapp.com/attachments/1420109753878839338/1427345064392134907/makesweet-ve36v1.gif?ex=6916bbee&is=69156a6e&hm=6c08b65797ebce7ee1638a2a6ef8ee2f6746503c05f7cdb70e061d7a571790d9&")
                .await?;
        }
        _ => {}
    }

    Ok(())
}
