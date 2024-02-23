use std::fmt::Display;

use chrono::{NaiveDate, NaiveDateTime};
use futures_util::{StreamExt, TryStreamExt};
use sea_orm::{ConnectionTrait, DbErr, Statement, StreamTrait};
use tgbot::{
    api::Client,
    types::{ParseMode, ReplyParameters, SendMessage},
};

use crate::Error;

use super::{Chat, Lang};

struct Stat {
    _id: i32,
    name: String,
    min_position: i32,
    max_position: i32,
    avg_position: i32,
    min_date: NaiveDate,
    max_date: NaiveDate,
    start_position: i32,
    end_position: i32,
    diff: i32,
    per_day: i32,
}

#[derive(Default)]
enum Sort {
    Min,
    Max,
    Avg,
    Diff,
    #[default]
    PerDay,
}

impl<'a> TryFrom<&'a str> for Sort {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "min" => Ok(Sort::Min),
            "max" => Ok(Sort::Max),
            "avg" => Ok(Sort::Avg),
            "diff" => Ok(Sort::Diff),
            "xday" | "algg" => Ok(Sort::PerDay),
            value => Err(value),
        }
    }
}

impl Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sort::Min => f.write_str("min"),
            Sort::Max => f.write_str("max"),
            Sort::Avg => f.write_str("avg"),
            Sort::Diff => f.write_str("diff"),
            Sort::PerDay => f.write_str("per_day"),
        }
    }
}

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    sort_by: Option<&str>,
    message_id: i64,
    chat: &Chat,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + StreamTrait,
{
    let sort = match sort_by.map(Sort::try_from).transpose() {
        Ok(sort) => sort.unwrap_or_default(),
        Err(s) => {
            return Ok(Err(match chat.lang {
                Lang::En => format!("Invalid sort field \"{s}\""),
                Lang::It => format!("Campo di ordinamento non valido \"{s}\""),
            }));
        }
    };

    // fucking ORM making complex queries a nightmare
    let query = format!("select sub.*, start.position, end.position, start.position - end.position as diff, (start.position - end.position) / (JulianDay(max_date) - JulianDay(min_date)) per_day
    from (select pp.id, pp.name, min(p.position) min, max(p.position) max, avg(p.position) avg, min(p.date) as min_date, max(p.date) as max_date
    from pornstars pp
    inner join positions p on pp.id = p.pornstar_id group by pp.id) sub
    inner join positions start on sub.id = start.pornstar_id AND start.date = sub.min_date
    inner join positions end on sub.id = end.pornstar_id and end.date = sub.max_date
    order by {sort} desc
    limit 10");

    let stmt = Statement::from_string(conn.get_database_backend(), query);
    let stats = conn
        .stream(stmt)
        .await?
        .map(|row| -> Result<Stat, DbErr> {
            let (
                _id,
                name,
                min_position,
                max_position,
                avg_position,
                min_date,
                max_date,
                start_position,
                end_position,
                diff,
                per_day,
            ) = row?.try_get_many_by_index::<(
                i32,
                String,
                i32,
                i32,
                f64,
                NaiveDateTime,
                NaiveDateTime,
                i32,
                i32,
                i32,
                Option<f64>,
            )>()?;

            Ok(Stat {
                _id,
                name,
                min_position,
                max_position,
                avg_position: avg_position.round() as i32,
                min_date: min_date.date(),
                max_date: max_date.date(),
                start_position,
                end_position,
                diff,
                per_day: per_day.unwrap_or_default().round() as i32,
            })
        })
        .try_collect::<Vec<_>>()
        .await?;

    let max_name_len = stats
        .iter()
        .map(|stat| stat.name.len())
        .max()
        .unwrap_or_default();

    client
        .execute(
            SendMessage::new(
                chat.id,
                format!(
                    "{}\n```\n{:max_name_len$}| min| max| avg|{:>15}|{:>15}|diff|{}\n{}```",
                    match chat.lang {
                        Lang::En => "Best performing 10",
                        Lang::It => "Migliori 10 per prestazioni",
                    },
                    match chat.lang {
                        Lang::En => "name",
                        Lang::It => "nome",
                    },
                    match chat.lang {
                        Lang::En => "first",
                        Lang::It => "primo",
                    },
                    match chat.lang {
                        Lang::En => "last",
                        Lang::It => "ultimo",
                    },
                    match chat.lang {
                        Lang::En => "xday",
                        Lang::It => "algg",
                    },
                    stats.into_iter().fold(String::new(), |acc, stat| {
                        let Stat {
                            _id,
                            name,
                            min_position,
                            max_position,
                            avg_position,
                            min_date,
                            max_date,
                            start_position,
                            end_position,
                            diff,
                            per_day
                        } = stat;

                        format!("{acc}{name:max_name_len$}|{min_position:4}|{max_position:4}|{avg_position:4}|{start_position:4}@{min_date}|{end_position:4}@{max_date}|{diff:4}|{per_day:4}\n")
                    })
                ),
            )
            .with_reply_parameters(ReplyParameters::new(message_id))
            .with_parse_mode(ParseMode::Markdown),
        )
        .await?;

    Ok(Ok(()))
}
