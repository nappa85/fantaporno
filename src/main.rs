use std::{env, sync::Arc};

use chrono::{Duration, OutOfRangeError, Timelike, Utc, Weekday};
use futures_util::future::try_join_all;
use reqwest::Client;
use scraper::{error::SelectorErrorKind, Html, Selector};
use sea_orm::{ConnectionTrait, Database, DbErr, TransactionTrait};
use tgbot::api::{ClientError, ExecuteError};
use tokio::{select, sync::Notify, time::sleep};

mod bot;
mod entities;

// const TOP_800_URL: &str = "https://www.pornhub.com/pornstars/top";
const PORNSTAR_AMATORIAL_URL: &str = "https://www.pornhub.com/pornstars?page=";
const USER_AGENT: &str = "Tua madre";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Missing env var BOT_TOKEN")]
    MissingBotToken,
    #[error("Missing env var BOT_NAME")]
    MissingBotName,
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
    let token = env::var("BOT_TOKEN").map_err(|_| Error::MissingBotToken)?;
    let name = env::var("BOT_NAME").map_err(|_| Error::MissingBotName)?;
    let name = format!("@{}", name.strip_prefix('@').unwrap_or(name.as_str()));
    let conn = Database::connect("sqlite:fantaporno.sqlite3").await?;
    let notify = Arc::new(Notify::new());
    let notify2 = Arc::clone(&notify);
    // let scraper = scrape_top_800;
    let scraper = scrape_pornstar_amatorial;
    select! {
        out = scraper(&conn, notify) => {
            println!("scraper terminated");
            out
        },
        out = bot::execute(&conn, token, &name, notify2) => {
            println!("bot terminated");
            out
        },
    }
}

// async fn scrape_top_800<C>(conn: &C, notifier: Arc<Notify>) -> Result<(), Error>
// where
//     C: ConnectionTrait + TransactionTrait,
// {
//     let client = Client::new();
//     let list_content = Selector::parse("ul#categoryListContent")?;
//     let rank = Selector::parse("li.index-length")?;
//     let name = Selector::parse("li.index-title a")?;

//     let mut error = false;
//     loop {
//         let now = Utc::now();
//         // on error simply wait one hour and retry, else wait next sunday
//         let next_tick = if error {
//             now.date_naive()
//                 .and_hms_opt(now.hour(), 0, 0)
//                 .ok_or(Error::InvalidNextHour)?
//         } else {
//             now.date_naive()
//                 .week(Weekday::Sun)
//                 .last_day()
//                 .and_hms_opt(23, 0, 0)
//                 .ok_or(Error::InvalidNextWeek)?
//         };
//         let next_tick = next_tick
//             .and_local_timezone(Utc)
//             .single()
//             .ok_or(Error::InvalidTimezone)?
//             + Duration::hours(1);
//         sleep((next_tick - now).to_std()?).await;

//         let response = client
//             .get(TOP_800_URL)
//             .header("User-Agent", USER_AGENT)
//             .send()
//             .await?;
//         let text = response.text().await?;
//         let doc = Html::parse_document(&text);

//         let txn = conn.begin().await?;
//         error = false;
//         let mut commit = false;
//         for element in doc.select(&list_content) {
//             let Some(rank_el) = element.select(&rank).next() else {
//                 commit = false;
//                 error = true;
//                 break;
//             };
//             let Some(rank) = rank_el
//                 .text()
//                 .next()
//                 .and_then(|rank| rank.trim().parse().ok())
//             else {
//                 commit = false;
//                 error = true;
//                 break;
//             };
//             let Some(name_el) = element.select(&name).next() else {
//                 commit = false;
//                 error = true;
//                 break;
//             };
//             let Some(name) = name_el.text().next().map(str::trim) else {
//                 commit = false;
//                 error = true;
//                 break;
//             };
//             let Some(url) = name_el.value().attr("href") else {
//                 commit = false;
//                 error = true;
//                 break;
//             };

//             let pornstar = entities::pornstar::find_or_insert(&txn, name, url).await?;

//             if entities::position::inserted(&txn, pornstar.id, next_tick, rank).await? {
//                 commit = true;
//             }
//         }

//         if commit {
//             txn.commit().await?;
//             notifier.notify_one();
//         } else {
//             txn.rollback().await?;
//         }
//     }
// }

async fn scrape_pornstar_amatorial<C>(conn: &C, notifier: Arc<Notify>) -> Result<(), Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let client = Client::new();
    let list_content = Selector::parse("ul#pornstarListSection li div.performerCard").unwrap();
    let rank = Selector::parse("span.rank_number").unwrap();
    let name = Selector::parse("img.pornstarThumb").unwrap();
    let link = Selector::parse("a.pornstarLink").unwrap();

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

        let txn = conn.begin().await?;
        let scraped = try_join_all((1..=16).map(|page| {
            scrape_pornstar_amatorial_page(&txn, &client, &list_content, &rank, &name, &link, page)
        }))
        .await?;
        let mut commit = false;
        for scrap in scraped {
            if let Some(pornstar_rank) = scrap {
                for (pornstar_id, rank) in pornstar_rank {
                    if entities::position::inserted(&txn, pornstar_id, next_tick, rank).await? {
                        commit = true;
                    }
                }
            } else {
                error = true;
                commit = false;
                break;
            }
        }

        if commit {
            txn.commit().await?;
            notifier.notify_one();
        } else {
            txn.rollback().await?;
        }
    }
}

async fn scrape_pornstar_amatorial_page<C>(
    conn: &C,
    client: &Client,
    list_content: &Selector,
    rank: &Selector,
    name: &Selector,
    link: &Selector,
    page: u8,
) -> Result<Option<Vec<(i32, i32)>>, Error>
where
    C: ConnectionTrait,
{
    let response = client
        .get(format!("{PORNSTAR_AMATORIAL_URL}{page}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let text = response.text().await?;
    let doc = Html::parse_document(&text);

    let mut ranks = Vec::new();
    for element in doc.select(list_content) {
        let Some(rank_el) = element.select(rank).next() else {
            return Ok(None);
        };
        let Some(rank) = rank_el
            .text()
            .next()
            .and_then(|rank| rank.trim().parse().ok())
        else {
            return Ok(None);
        };
        let Some(name_el) = element.select(name).next() else {
            return Ok(None);
        };
        let Some(name) = name_el.value().attr("alt") else {
            return Ok(None);
        };
        let Some(link_el) = element.select(link).next() else {
            return Ok(None);
        };
        let Some(url) = link_el.value().attr("href") else {
            return Ok(None);
        };

        let pornstar = entities::pornstar::find_or_insert(conn, name, url).await?;

        ranks.push((pornstar.id, rank));
    }

    Ok(Some(ranks))
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use scraper::Selector;
    use sea_orm::Database;

    #[tokio::test]
    async fn scraper() {
        let conn = Database::connect("sqlite:fantaporno.sqlite3")
            .await
            .unwrap();
        let client = Client::new();
        let list_content = Selector::parse("ul#pornstarListSection li div.performerCard").unwrap();
        let rank = Selector::parse("span.rank_number").unwrap();
        let name = Selector::parse("img.pornstarThumb").unwrap();
        let link = Selector::parse("a.pornstarLink").unwrap();
        let out = super::scrape_pornstar_amatorial_page(
            &conn,
            &client,
            &list_content,
            &rank,
            &name,
            &link,
            1,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(out[0].1, 1);
    }
}
