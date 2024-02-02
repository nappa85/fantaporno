use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use tgbot::{
    api::Client,
    types::{ReplyParameters, SendMessage, User},
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
    if crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(i64::from(user.id))
                .and(crate::entities::player::Column::ChatId.eq(chat.id)),
        )
        .one(conn)
        .await?
        .is_some()
    {
        return Ok(Err(String::from(match chat.lang {
            Lang::En => "Player already exists",
            Lang::It => "Il giocatore esiste già",
        })));
    }

    crate::entities::player::insert(conn, user, chat.id).await?;

    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => "Player created",
                    Lang::It => "Giocatore creato",
                },
            )
            .with_reply_parameters(ReplyParameters::new(message_id)),
        )
        .await?;

    Ok(Ok(()))
}
