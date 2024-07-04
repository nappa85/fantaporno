use std::borrow::Cow;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QuerySelect, TransactionTrait,
};
use tgbot::{
    api::Client,
    types::{ParseMode, ReplyParameters, SendMessage, User},
};

use crate::Error;

use super::{Chat, Lang};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    message_id: i64,
    chat: &Chat,
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let player = match crate::entities::player::find(conn, user, chat.id, chat.lang).await? {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name, chat.lang).await? {
        Ok(pornstar) => pornstar,
        Err(err) => return Ok(Err(err)),
    };

    let now = Utc::now().naive_utc();
    if let Some(team) = crate::entities::team::Entity::find()
        .filter(
            crate::entities::team::Column::PornstarId
                .eq(pornstar.id)
                .and(
                    crate::entities::team::Column::EndDate
                        .is_null()
                        .or(crate::entities::team::Column::EndDate.gt(now)),
                ),
        )
        .one(conn)
        .await?
    {
        return Ok(Err(match chat.lang {
            Lang::En => format!(
                "Pornstar \"{}\" is already in {} team",
                pornstar.link(),
                if team.player_id == player.id {
                    Cow::Borrowed("your")
                } else if let Some(owner) =
                    crate::entities::player::Entity::find_by_id(team.player_id)
                        .one(conn)
                        .await?
                {
                    Cow::Owned(format!("{}'s", owner.tg_link()))
                } else {
                    Cow::Borrowed("another")
                }
            ),
            Lang::It => format!(
                "Il/la pornostar \"{}\" è già {}",
                pornstar.link(),
                if team.player_id == player.id {
                    Cow::Borrowed("nella tua squadra")
                } else if let Some(owner) =
                    crate::entities::player::Entity::find_by_id(team.player_id)
                        .one(conn)
                        .await?
                {
                    Cow::Owned(format!("nella squadra di {}", owner.tg_link()))
                } else {
                    Cow::Borrowed("in un'altra squadra")
                }
            ),
        }));
    }

    let cost = match pornstar.get_cost(conn, chat.lang).await? {
        Ok(cost) => cost,
        Err(err) => return Ok(Err(err)),
    };
    if cost > player.budget {
        return Ok(Err(match chat.lang {
            Lang::En => format!(
                "You don't have enough balance to buy \"{}\"",
                pornstar.link()
            ),
            Lang::It => format!(
                "Non hai abbastanza soldi per comprare \"{}\"",
                pornstar.link()
            ),
        }));
    }

    let team_size = crate::entities::team::Entity::find()
        .filter(
            crate::entities::team::Column::PlayerId.eq(player.id).and(
                crate::entities::team::Column::EndDate
                    .is_null()
                    .or(crate::entities::team::Column::EndDate.gt(now)),
            ),
        )
        .select_only()
        .column_as(crate::entities::team::Column::PornstarId.count(), "count")
        .into_tuple::<i64>()
        .one(conn)
        .await?;

    if team_size.unwrap_or_default() >= super::MAX_TEAM_SIZE {
        return Ok(Err(match chat.lang {
            Lang::En => "Your team already is of the max size".to_owned(),
            Lang::It => "La tua squadra è già della dimensione massima".to_owned(),
        }));
    }

    let txn = conn.begin().await?;

    crate::entities::team::ActiveModel {
        player_id: ActiveValue::Set(player.id),
        pornstar_id: ActiveValue::Set(pornstar.id),
        start_date: ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    crate::entities::player::ActiveModel {
        id: ActiveValue::Set(player.id),
        budget: ActiveValue::Set(player.budget - cost),
        ..Default::default()
    }
    .update(&txn)
    .await?;

    txn.commit().await?;

    client
        .execute(
            SendMessage::new(
                chat.id,
                match chat.lang {
                    Lang::En => format!("Pornstar \"{}\" now is in your team", pornstar.link()),
                    Lang::It => format!(
                        "Il/la pornostar \"{}\" ora è nella tua squadra",
                        pornstar.link()
                    ),
                },
            )
            .with_parse_mode(ParseMode::Markdown)
            .with_reply_parameters(ReplyParameters::new(message_id)),
        )
        .await?;

    Ok(Ok(()))
}
