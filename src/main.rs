use std::{env, sync::Arc};

use fantaporno::{bot, scrape_pornstar_amatorial, Error};
use sea_orm::Database;
use tokio::{select, sync::Notify};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let token = env::var("BOT_TOKEN").map_err(|_| Error::MissingBotToken)?;
    let name = env::var("BOT_NAME").map_err(|_| Error::MissingBotName)?;
    let name = format!("@{}", name.strip_prefix('@').unwrap_or(name.as_str()));
    let conn = Database::connect("sqlite:fantaporno.sqlite3").await?;
    let notify = Arc::new(Notify::new());
    let notify2 = Arc::clone(&notify);
    select! {
        out = scrape_pornstar_amatorial(&conn, notify) => {
            println!("scraper terminated");
            out
        },
        out = bot::execute(&conn, token, &name, notify2) => {
            println!("bot terminated");
            out
        },
    }
}
