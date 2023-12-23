use sea_orm::ConnectionTrait;
use tgbot::{
    api::Client,
    types::{SendMessage, User},
};

use crate::Error;

use super::{Chat, Lang};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    message_id: i64,
    chat: &Chat,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait,
{
    let player = match crate::entities::player::find(conn, user.id, chat.id, chat.lang).await? {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => format!("Your remaining budget is {}€", player.budget),
                    Lang::It => format!("Il tuo budget rimanente è {}€", player.budget),
                },
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
