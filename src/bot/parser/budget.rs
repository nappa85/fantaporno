use sea_orm::{ConnectionTrait, EntityTrait};
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
    let Ok(user_id) = u32::try_from(i64::from(user.id)) else {
        return Ok(Err(format!("Invalid user id: {}", user.id)));
    };

    let Some(player) = crate::entities::player::Entity::find_by_id(user_id)
        .one(conn)
        .await?
    else {
        return Ok(Err("Player doesn't exists".into()));
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
