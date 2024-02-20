use sea_orm::{ConnectionTrait, StreamTrait};
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
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + StreamTrait,
{
    let player = match crate::entities::player::find(conn, user, chat.id, chat.lang).await? {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name, chat.lang).await? {
        Ok(pornstar) => pornstar,
        Err(err) => return Ok(Err(err)),
    };

    let mut history = player.history(conn, Some([pornstar.id])).await?;

    let msg = if let Some(positions) = history.remove(&pornstar.id) {
        positions.scores().rev().take(20).fold(
            match chat.lang {
                Lang::En => format!("Pornstar \"{}\" last 20 contributions:", pornstar.name),
                Lang::It => format!(
                    "Ultimi 20 punteggi del/della pornostar \"{}\":",
                    pornstar.name
                ),
            },
            |mut buf, (date, points)| {
                buf.push('\n');
                buf.push_str(&date.format("%Y-%m-%d").to_string());
                buf.push(' ');
                if points >= 0 {
                    buf.push('+');
                }
                buf.push_str(&points.to_string());
                buf.push_str(match chat.lang {
                    Lang::En => " points",
                    Lang::It => " punti",
                });
                buf
            },
        )
    } else {
        match chat.lang {
            Lang::En => {
                format!("Pornstar \"{}\" never made points for you", pornstar.name)
            }
            Lang::It => format!(
                "Il/la pornostar \"{}\" non ha mai generato punti per te",
                pornstar.name
            ),
        }
    };

    client
        .execute(
            SendMessage::new(chat.id, msg).with_reply_parameters(ReplyParameters::new(message_id)),
        )
        .await?;

    Ok(Ok(()))
}
