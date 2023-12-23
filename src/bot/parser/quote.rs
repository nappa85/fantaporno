use sea_orm::{ConnectionTrait, TransactionTrait};
use tgbot::{api::Client, types::SendMessage};

use crate::Error;

use super::{Chat, Lang};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    message_id: i64,
    chat: &Chat,
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name, chat.lang).await? {
        Ok(pornstar) => pornstar,
        Err(err) => return Ok(Err(err)),
    };

    let cost = match pornstar.get_cost(conn, chat.lang).await? {
        Ok(cost) => cost,
        Err(err) => return Ok(Err(err)),
    };

    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => format!("Pornstar \"{}\" value is {cost}€", pornstar.name),
                    Lang::It => format!(
                        "Il valore del/della pornostar \"{}\" è {cost}€",
                        pornstar.name
                    ),
                },
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
