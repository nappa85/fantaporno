use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, StreamTrait, TransactionTrait,
};
use tgbot::{
    api::Client,
    types::{
        ChatPeerId, ParseMode, ReplyParameters, SendMessage, Text, TextEntities, TextEntity,
        TextEntityPosition, User,
    },
};

use crate::entities::{
    chat::{Lang, Model as Chat},
    player,
};

const MAX_TEAM_SIZE: i64 = 11;

mod budget;
mod buy;
pub mod chart;
mod chat;
mod create;
mod help;
mod history;
mod quote;
mod sell;
mod stats;
mod team;

pub async fn parse_message<C>(
    client: &Client,
    conn: &C,
    name: &str,
    user: &User,
    message_id: i64,
    msg: &Text,
    chat_id: ChatPeerId,
) -> Result<(), crate::Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let chat = crate::entities::chat::find_or_insert(conn, chat_id).await?;

    let Text {
        data: msg,
        entities,
    } = msg;

    let mut iter = msg.split_whitespace();
    let res = match iter.next().map(|msg| msg.strip_suffix(name).unwrap_or(msg)) {
        Some("/help") => help::execute(client, message_id, &chat).await.map(Ok)?,
        Some("/start") => create::execute(client, conn, user, message_id, &chat).await?,
        Some("/budget") => budget::execute(client, conn, user, message_id, &chat).await?,
        Some("/team") => {
            let tag = entities
                .as_ref()
                .and_then(|entities| Tag::parse_one(msg, entities));
            team::execute(client, conn, user, tag, message_id, &chat).await?
        }
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
        Some("/history") => {
            let pornstar_name = iter.fold(String::new(), |mut buf, chunk| {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(chunk);
                buf
            });

            history::execute(client, conn, user, message_id, &chat, pornstar_name).await?
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
        Some("/stats") => {
            let field = iter.next();
            stats::execute(client, conn, field, message_id, &chat).await?
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
                .with_reply_parameters(ReplyParameters::new(message_id))
                .with_parse_mode(ParseMode::Markdown),
            )
            .await?;
    }

    Ok(())
}

enum Tag<'a> {
    Username(&'a str),
    UserId(&'a str, i64),
}

impl<'a> Tag<'a> {
    fn parse_one(msg: &'a str, entities: &'a TextEntities) -> Option<Self> {
        entities
            .into_iter()
            .filter_map(|entity| match entity {
                TextEntity::Mention(TextEntityPosition { offset, length }) => Some(Tag::Username(
                    &msg[usize::try_from(*offset).unwrap()
                        ..usize::try_from(*offset + *length).unwrap()],
                )),
                TextEntity::TextMention {
                    position: TextEntityPosition { offset, length },
                    user,
                    ..
                } => Some(Tag::UserId(
                    &msg[usize::try_from(*offset).unwrap()
                        ..usize::try_from(*offset + *length).unwrap()],
                    i64::from(user.id),
                )),
                _ => None,
            })
            .next()
    }

    async fn load_player<C: ConnectionTrait>(
        &self,
        conn: &C,
        chat_id: i64,
        lang: Lang,
    ) -> Result<Result<player::Model, String>, DbErr> {
        let (tag, expr) = match self {
            Tag::Username(tag) => (*tag, crate::entities::player::Column::Tag.like(&tag[1..])),
            Tag::UserId(tag, id) => (*tag, crate::entities::player::Column::TelegramId.eq(*id)),
        };
        let Some(player) = player::Entity::find()
            .filter(expr.and(crate::entities::player::Column::ChatId.eq(chat_id)))
            .one(conn)
            .await?
        else {
            return Ok(Err(match lang {
                Lang::En => format!("Player {tag} doesn't exists"),
                Lang::It => format!("Il giocatore {tag} non esiste"),
            }));
        };

        Ok(Ok(player))
    }
}
