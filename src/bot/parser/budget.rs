use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use tgbot::{
    api::Client,
    types::{ChatPeerId, SendMessage, User},
};

use crate::Error;

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    message_id: i64,
    chat_id: ChatPeerId,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait,
{
    let Some(player) = crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(i64::from(user.id))
                .and(crate::entities::player::Column::ChatId.eq(i64::from(chat_id))),
        )
        .one(conn)
        .await?
    else {
        return Ok(Err("Player doesn't exists, use /start to create".into()));
    };

    client
        .execute(
            SendMessage::new(
                chat_id,
                format!("Your remaining budget is {}€", player.budget),
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
