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
    if crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(i64::from(user.id))
                .and(crate::entities::player::Column::ChatId.eq(i64::from(chat_id))),
        )
        .one(conn)
        .await?
        .is_some()
    {
        return Ok(Err("Player already exists".into()));
    }

    crate::entities::player::insert(
        conn,
        i64::from(user.id),
        i64::from(chat_id),
        if let Some(last_name) = &user.last_name {
            format!("{} {last_name}", user.first_name)
        } else {
            user.first_name.clone()
        },
    )
    .await?;

    client
        .execute(SendMessage::new(chat_id, "Player created").with_reply_to_message_id(message_id))
        .await?;

    Ok(Ok(()))
}
