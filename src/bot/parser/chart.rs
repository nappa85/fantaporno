use chrono::Utc;
use sea_orm::{ConnectionTrait, EntityTrait, StreamTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, ParseMode, SendMessage},
};

use crate::Error;

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    message_id: Option<i64>,
    chat_id: ChatPeerId,
) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait,
{
    // we can't use `stream` here since `score` would deadlock the connection
    let players = crate::entities::player::Entity::find().all(conn).await?;
    let msg = if players.is_empty() {
        String::from("At the moment there are no players, use /start to join")
    } else {
        let now = Utc::now().naive_utc();
        let mut scores = Vec::with_capacity(players.len());
        for player in players {
            let score = player.score(conn, now).await?;
            scores.push((player, score));
        }
        scores.sort_unstable_by(|a, b| a.1.cmp(&b.1));

        scores
            .into_iter()
            .fold(String::new(), |mut buf, (player, score)| {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&format!(
                    "[{}](tg://user?id={}) {score}",
                    player.name, player.telegram_id
                ));
                buf
            })
    };

    let message = SendMessage::new(chat_id, msg).with_parse_mode(ParseMode::Markdown);
    let message = if let Some(message_id) = message_id {
        message.with_reply_to_message_id(message_id)
    } else {
        message
    };
    client.execute(message).await?;

    Ok(())
}
