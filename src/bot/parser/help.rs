use tgbot::{
    api::Client,
    types::{ChatPeerId, SendMessage},
};

use crate::Error;

const HELP_MESSAGE: &str = "/budget - player budget
/buy {pornstar} - buy given pornstar
/chart - show players chart
/help - this message
/quote {pornstar} - quote given pornstar
/sell {pornstar} - sell given pornstar
/start - create account
/team - show player team";

pub async fn execute(client: &Client, message_id: i64, chat_id: ChatPeerId) -> Result<(), Error> {
    client
        .execute(SendMessage::new(chat_id, HELP_MESSAGE).with_reply_to_message_id(message_id))
        .await?;

    Ok(())
}
