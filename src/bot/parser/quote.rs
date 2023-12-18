use sea_orm::{ConnectionTrait, TransactionTrait};
use tgbot::{
    api::Client,
    types::{ChatPeerId, SendMessage},
};

use crate::Error;

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    message_id: i64,
    chat_id: ChatPeerId,
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name).await {
        Ok(Ok(pornstar)) => pornstar,
        Ok(Err(err)) => return Ok(Err(err)),
        Err(err) => return Err(Error::from(err)),
    };

    let Some(cost) = pornstar.get_cost(conn).await? else {
        return Ok(Err(format!(
            "Pornstar \"{}\" doesn't have a valutation at the moment",
            pornstar.name
        )));
    };

    client
        .execute(
            SendMessage::new(
                chat_id,
                format!("Pornstar \"{}\" value is {cost}€", pornstar.name),
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
