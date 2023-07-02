use chrono::{Duration, Timelike, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use sea_orm::{Database, TransactionTrait};
use tokio::time::sleep;

mod entities;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Sea-orm error: {0}")]
    SeaOrm(#[from] sea_orm::DbErr),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conn = Database::connect("sqlite:fantaporno.sqlite3").await?;
    let client = Client::new();
    let list_content = Selector::parse("ul#categoryListContent").unwrap();
    let rank = Selector::parse("li.index-length").unwrap();
    let name = Selector::parse("li.index-title a").unwrap();

    loop {
        let now = Utc::now();
        let next_hour =
            now.with_second(0).unwrap().with_nanosecond(0).unwrap() + Duration::hours(1);
        sleep((next_hour - now).to_std().unwrap()).await;

        let response = client
            .get("https://www.pornhub.com/pornstars/top")
            .header("User-Agent", "Tua madre")
            .send()
            .await?;
        let text = response.text().await?;
        let doc = Html::parse_document(&text);

        let txn = conn.begin().await?;
        let mut commit = false;
        for element in doc.select(&list_content) {
            let rank_el = element.select(&rank).next().unwrap();
            let rank = rank_el.text().next().unwrap().trim().parse().unwrap();
            let name_el = element.select(&name).next().unwrap();
            let name = name_el.text().next().unwrap().trim();
            let url = name_el.value().attr("href").unwrap();

            let pornstar = entities::pornstar::find_or_insert(&txn, name, url).await?;

            if entities::position::inserted(&txn, pornstar.id, next_hour, rank).await? {
                commit = true;
            }
        }

        if commit {
            txn.commit().await?;
        } else {
            txn.rollback().await?;
        }
    }
}
