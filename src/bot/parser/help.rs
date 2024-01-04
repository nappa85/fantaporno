use tgbot::{
    api::Client,
    types::{ParseMode, ReplyParameters, SendMessage},
};

use crate::Error;

use super::{Chat, Lang};

const HELP_MESSAGE_EN: &str = r#"<a href="https://github.com/nappa85/fantaporno/">Fantaporno Bot</a>

/budget - player budget
/buy {pornstar} - buy given pornstar
/chart - show players chart
/help - this message
/history {pornstar} - show last 20 contributions of given pornstar for player's team
/quote {pornstar} - quote given pornstar
/sell {pornstar} - sell given pornstar
/set_chat_lang {lang} - set given lang for this chat (at the moment supports only "en" and "it")
/start - create account
/team - show player's team"#;

const HELP_MESSAGE_IT: &str = r#"<a href="https://github.com/nappa85/fantaporno/">Fantaporno Bot</a>

/budget - budget del giocatore
/buy {pornostar} - compra il/la pornostar
/chart - mostra la classifica giocatori
/help - questo messaggio
/history {pornostar} - mostra gli ultimi 20 punteggi del/della pornostar per la squadra del giocatore
/quote {pornostar} - valuta il/la pornostar
/sell {pornostar} - vendi il/la pornostar
/set_chat_lang {lingua} - imposta la lingua per questa chat (al momento supporta solo "en" e "it")
/start - crea giocatore
/team - mostra la squadra del giocatore"#;

pub async fn execute(client: &Client, message_id: i64, chat: &Chat) -> Result<(), Error> {
    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => HELP_MESSAGE_EN,
                    Lang::It => HELP_MESSAGE_IT,
                },
            )
            .with_reply_parameters(ReplyParameters::new(message_id))
            .with_parse_mode(ParseMode::Html),
        )
        .await?;

    Ok(())
}
