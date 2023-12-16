use std::{env, sync::Arc};

use chrono::{Duration, OutOfRangeError, Timelike, Utc, Weekday};
use reqwest::Client;
use scraper::{error::SelectorErrorKind, Html, Selector};
use sea_orm::{ConnectionTrait, Database, DbErr, TransactionTrait};
use tgbot::{
    api::{ClientError, ExecuteError},
    types::ChatPeerId,
};
use tokio::{select, sync::Notify, time::sleep};

mod bot;
mod entities;

const RANK_URL: &str = "https://www.pornhub.com/pornstars/top";
const USER_AGENT: &str = "Tua madre";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Env var error: {0}")]
    Env(#[from] env::VarError),
    #[error("Invalid CHAT_ID")]
    InvalidChatId,
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Sea-orm error: {0}")]
    SeaOrm(#[from] DbErr),
    #[error("Scraper error: {0}")]
    Scraper(#[from] SelectorErrorKind<'static>),
    #[error("Invalid next hour")]
    InvalidNextHour,
    #[error("Invalid next week")]
    InvalidNextWeek,
    #[error("Invalid timezone")]
    InvalidTimezone,
    #[error("Chrono error: {0}")]
    Chrono(#[from] OutOfRangeError),
    #[error("Telegram client error: {0}")]
    TelegramClient(#[from] ClientError),
    #[error("Telegram execute error: {0}")]
    TelegramExec(#[from] ExecuteError),
    #[error("Invalid position")]
    InvalidPosition,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("BOT_TOKEN")?;
    let chat_id: i64 = env::var("CHAT_ID")?
        .parse()
        .map_err(|_| Error::InvalidChatId)?;
    let conn = Database::connect("sqlite:fantaporno.sqlite3").await?;
    let notify = Arc::new(Notify::new());
    let notify2 = Arc::clone(&notify);
    select! {
        out = scraper(&conn, notify) => {
            println!("scraper terminated");
            out
        },
        out = bot::execute(&conn, token, notify2, ChatPeerId::from(chat_id)) => {
            println!("bot terminated");
            out
        },
    }
}

async fn scraper<C>(conn: &C, notifier: Arc<Notify>) -> Result<(), Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let client = Client::new();
    let list_content = Selector::parse("ul#categoryListContent")?;
    let rank = Selector::parse("li.index-length")?;
    let name = Selector::parse("li.index-title a")?;

    let mut error = false;
    loop {
        let now = Utc::now();
        // on error simply wait one hour and retry, else wait next sunday
        let next_tick = if error {
            now.date_naive()
                .and_hms_opt(now.hour(), 0, 0)
                .ok_or(Error::InvalidNextHour)?
        } else {
            now.date_naive()
                .week(Weekday::Sun)
                .last_day()
                .and_hms_opt(23, 0, 0)
                .ok_or(Error::InvalidNextWeek)?
        };
        let next_tick = next_tick
            .and_local_timezone(Utc)
            .single()
            .ok_or(Error::InvalidTimezone)?
            + Duration::hours(1);
        sleep((next_tick - now).to_std()?).await;

        let response = client
            .get(RANK_URL)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;
        let text = response.text().await?;
        let doc = Html::parse_document(&text);

        let txn = conn.begin().await?;
        error = false;
        let mut commit = false;
        for element in doc.select(&list_content) {
            let Some(rank_el) = element.select(&rank).next() else {
                commit = false;
                break;
            };
            let Some(rank) = rank_el
                .text()
                .next()
                .and_then(|rank| rank.trim().parse().ok())
            else {
                commit = false;
                break;
            };
            let Some(name_el) = element.select(&name).next() else {
                commit = false;
                break;
            };
            let Some(name) = name_el.text().next().map(str::trim) else {
                commit = false;
                break;
            };
            let Some(url) = name_el.value().attr("href") else {
                commit = false;
                break;
            };

            let pornstar = entities::pornstar::find_or_insert(&txn, name, url).await?;

            if entities::position::inserted(&txn, pornstar.id, next_tick, rank).await? {
                commit = true;
            }
        }

        if commit {
            txn.commit().await?;
            notifier.notify_one();
        } else {
            txn.rollback().await?;
            error = true;
        }
    }
}
