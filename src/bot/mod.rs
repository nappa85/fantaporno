use std::{sync::Arc, time::Duration};

use crate::{entities::chat, Error};
use sea_orm::{ConnectionTrait, EntityTrait, StreamTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{GetUpdates, Message, MessageData, UpdateType},
};
use tokio::{select, sync::Notify};

mod parser;

pub async fn execute<C>(
    conn: &C,
    token: String,
    name: &str,
    notifier: Arc<Notify>,
) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let client = Client::new(token)?;
    select! {
        out = receiver(&client, conn, name) => out,
        out = sender(&client, conn, notifier) => out,
    }
}

async fn receiver<C>(client: &Client, conn: &C, name: &str) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let mut offset = -1;
    loop {
        let updates = client
            .execute(
                GetUpdates::default()
                    .with_timeout(Duration::from_secs(3600))
                    .with_offset(offset + 1),
            )
            .await?;
        for update in updates {
            let Some(chat_id) = update.get_chat_id() else {
                continue;
            };
            let Some(user) = update.get_user() else {
                continue;
            };

            if let UpdateType::Message(Message {
                id,
                data: MessageData::Text(ref msg),
                ..
            }) = update.update_type
            {
                parser::parse_message(client, conn, name, user, id, msg, chat_id).await?
            } else {
                println!("Update {update:?}");
            }
            offset = update.id;
        }
    }
}

async fn sender<C>(client: &Client, conn: &C, notifier: Arc<Notify>) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait,
{
    loop {
        notifier.notified().await;

        // we can't use `stream` here since `chart::execute` would deadlock the connection
        let chats = chat::Entity::find().all(conn).await?;

        for chat in chats {
            let _ = parser::chart::execute(client, conn, None, &chat).await?;
        }
    }
}
