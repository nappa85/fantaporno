use std::time::Duration;

use crate::{entities::chat, Error};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, StreamTrait, TransactionTrait,
};
use tgbot::{
    api::{Client, ExecuteError},
    types::{GetUpdates, Message, MessageData, UpdateType},
};
use tokio::{select, sync::Notify, time};
use tracing::{debug, error, warn};

mod parser;

pub async fn execute<C>(conn: &C, token: String, name: &str, notifier: &Notify) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let client = Client::new(token)?;
    select! {
        out = receiver(&client, conn, name) => out,
        out = sender(&client, conn, notifier) => out,
    }
}

// ignores non-fatal errors
fn clear_error(res: Result<(), Error>) -> Result<(), Error> {
    if let Err(Error::TelegramExec(ExecuteError::Response(response_error))) = &res {
        if response_error.error_code() == Some(400) {
            warn!("Ignoring Telegram error: {response_error}");
            return Ok(());
        }
    }
    res
}

async fn receiver<C>(client: &Client, conn: &C, name: &str) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait + TransactionTrait,
{
    let mut offset = -1;
    loop {
        let updates = match client
            .execute(
                GetUpdates::default()
                    .with_timeout(Duration::from_secs(3600))
                    .with_offset(offset + 1),
            )
            .await
        {
            Ok(updates) => updates,
            Err(err) => {
                error!("Telegram poll error: {err}");
                time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

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
                clear_error(
                    parser::parse_message(client, conn, name, user, id, msg, chat_id).await,
                )?;
            } else {
                debug!("Ignoring update {update:?}");
            }
            offset = update.id;
        }
    }
}

async fn sender<C>(client: &Client, conn: &C, notifier: &Notify) -> Result<(), Error>
where
    C: ConnectionTrait + StreamTrait,
{
    loop {
        notifier.notified().await;

        // we can't use `stream` here since `chart::execute` would deadlock the connection
        let chats = chat::Entity::find().all(conn).await?;

        for chat in chats {
            if let Err(err) = parser::chart::execute(client, conn, None, &chat).await {
                if let super::Error::TelegramExec(ExecuteError::Response(response_error)) = &err {
                    // if response_error.description() == "Forbidden: bot was blocked by the user" {
                    if response_error.error_code() == Some(403) {
                        chat::ActiveModel {
                            id: ActiveValue::Set(chat.id),
                            ..Default::default()
                        }
                        .delete(conn)
                        .await?;
                        continue;
                    }
                }
                return Err(err);
            }
        }
    }
}
