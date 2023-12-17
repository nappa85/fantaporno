use std::{sync::Arc, time::Duration};

use crate::{entities::player, Error};
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect, StreamTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, GetUpdates, Message, MessageData, Text, UpdateType},
};
use tokio::{select, sync::Notify};

mod parser;

pub async fn execute<C>(conn: &C, token: String, notifier: Arc<Notify>) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let client = Client::new(token)?;
    select! {
        out = receiver(&client, conn) => out,
        out = sender(&client, conn, notifier) => out,
    }
}

async fn receiver<C>(client: &Client, conn: &C) -> Result<(), Error>
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

async fn sender<C>(client: &Client, conn: &C, notifier: Arc<Notify>) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait,
{
    loop {
        notifier.notified().await;

        let chat_ids = player::Entity::find()
            .select_only()
            .column(player::Column::ChatId)
            .distinct()
            .into_tuple::<i32>()
            .all(conn)
            .await?;

        for chat_id in chat_ids {
            let _ =
                parser::chart::execute(client, conn, None, ChatPeerId::from(i64::from(chat_id)))
                    .await?;
        }
    }
}
