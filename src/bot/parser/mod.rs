use sea_orm::{ConnectionTrait, StreamTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, ParseMode, SendMessage, User},
};

use crate::entities::chat::{Lang, Model as Chat};

mod budget;
mod buy;
pub mod chart;
mod chat;
mod create;
mod help;
mod quote;
mod sell;
mod team;

pub async fn parse_message<C>(
    client: &Client,
    conn: &C,
    name: &str,
    user: &User,
    message_id: i64,
    msg: &str,
    chat_id: ChatPeerId,
) -> Result<(), crate::Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let chat = crate::entities::chat::find_or_insert(conn, chat_id).await?;

    let mut iter = msg.split_whitespace();
    let res = match iter.next().map(|msg| msg.strip_suffix(name).unwrap_or(msg)) {
        Some("/help") => help::execute(client, message_id, &chat).await.map(Ok)?,
        Some("/start") => create::execute(client, conn, user, message_id, &chat).await?,
        Some("/budget") => budget::execute(client, conn, user, message_id, &chat).await?,
        Some("/team") => team::execute(client, conn, user, message_id, &chat).await?,
        Some("/chart") => chart::execute(client, conn, Some(message_id), &chat).await?,
        Some("/quote") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            quote::execute(client, conn, message_id, &chat, pornstar_name).await?
        }
        Some("/buy") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            buy::execute(client, conn, user, message_id, &chat, pornstar_name).await?
        }
        Some("/sell") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            sell::execute(client, conn, user, message_id, &chat, pornstar_name).await?
        }
        Some("/set_chat_lang") => {
            let lang = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            chat::execute(client, conn, message_id, &chat, lang).await?
        }
        _ => return Ok(()),
    };

    if let Err(err) = res {
        client
            .execute(
                SendMessage::new(
                    chat_id,
                    match chat.lang {
                        Lang::En => format!("Error: {err}"),
                        Lang::It => format!("Errore: {err}"),
                    },
                )
                .with_reply_to_message_id(message_id)
                .with_parse_mode(ParseMode::Markdown),
            )
            .await?;
    }

    Ok(())
}
