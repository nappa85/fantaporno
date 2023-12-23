use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait};
use tgbot::{api::Client, types::SendMessage};

use crate::Error;

use super::{Chat, Lang};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    message_id: i64,
    chat: &Chat,
    lang: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait,
{
    let Ok(lang) = crate::entities::chat::Lang::try_from(lang.as_str()) else {
        return Ok(Err(match chat.lang {
            Lang::En => format!("Invalid lang \"{}\"", lang),
            Lang::It => format!("Lingua non valida \"{}\"", lang),
        }));
    };

    let chat = crate::entities::chat::ActiveModel {
        id: ActiveValue::Set(chat.id),
        lang: ActiveValue::Set(lang),
        ..Default::default()
    }
    .update(conn)
    .await?;

    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => "Language set",
                    Lang::It => "Lingua impostata",
                },
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
