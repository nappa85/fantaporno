use sea_orm::{ConnectionTrait, StreamTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, SendMessage, User},
};

mod budget;
mod buy;
pub mod chart;
mod create;
mod help;
mod quote;
mod sell;
mod team;

pub async fn parse_message<C>(
    client: &Client,
    conn: &C,
    user: Option<&User>,
    message_id: i64,
    msg: &str,
    chat_id: ChatPeerId,
) -> Result<(), crate::Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let Some(user) = user else {
        // just ignore
        return Ok(());
    };

    let mut iter = msg.split_whitespace();
    let res = match iter.next() {
        Some("/help") => help::execute(client, message_id, chat_id).await.map(Ok)?,
        Some("/start") => create::execute(client, conn, user, message_id, chat_id).await?,
        Some("/budget") => budget::execute(client, conn, user, message_id, chat_id).await?,
        Some("/team") => team::execute(client, conn, user, message_id, chat_id).await?,
        Some("/chart") => chart::execute(client, conn, Some(message_id), chat_id).await?,
        Some("/quote") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            quote::execute(client, conn, message_id, chat_id, pornstar_name).await?
        }
        Some("/buy") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            buy::execute(client, conn, user, message_id, chat_id, pornstar_name).await?
        }
        Some("/sell") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            sell::execute(client, conn, user, message_id, chat_id, pornstar_name).await?
        }
        _ => return Ok(()),
    };

    if let Err(err) = res {
        client
            .execute(
                SendMessage::new(chat_id, format!("Error: {err}"))
                    .with_reply_to_message_id(message_id),
            )
            .await?;
    }

    Ok(())
}
