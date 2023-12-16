use std::{sync::Arc, time::Duration};

use crate::Error;
use sea_orm::{ConnectionTrait, StreamTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, GetUpdates, Message, MessageData, Text, UpdateType},
};
use tokio::{select, sync::Notify};

mod parser;

pub async fn execute<C>(
    conn: &C,
    token: String,
    notifier: Arc<Notify>,
    chat_id: ChatPeerId,
) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let client = Client::new(token)?;
    select! {
        out = receiver(&client, conn, chat_id) => out,
        out = sender(&client, conn, notifier, chat_id) => out,
    }
}

async fn receiver<C>(client: &Client, conn: &C, chat_id: ChatPeerId) -> Result<(), Error>
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
            // work only on designed chat
            if update.get_chat_id() != Some(chat_id) {
                continue;
            }

            println!("Update {update:?}");
            if let UpdateType::Message(Message {
                id,
                data: MessageData::Text(Text { ref data, .. }),
                ..
            }) = update.update_type
            {
                parser::parse_message(client, conn, update.get_user(), id, data, chat_id).await?
            }
            offset = update.id;
        }
    }
}

async fn sender<C>(
    client: &Client,
    conn: &C,
    notifier: Arc<Notify>,
    chat_id: ChatPeerId,
) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait,
{
    loop {
        notifier.notified().await;

        parser::chart::execute(client, conn, None, chat_id).await?;
    }
}
