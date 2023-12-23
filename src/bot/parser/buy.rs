use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use tgbot::{
    api::Client,
    types::{SendMessage, User},
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
    let player = match crate::entities::player::find(conn, user.id, chat.id, chat.lang).await? {
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
                pornstar.name,
                if team.player_id == player.id {
                    "your"
                } else {
                    "another"
                }
            ),
            Lang::It => format!(
                "Il/la pornostar \"{}\" è già {} squadra",
                pornstar.name,
                if team.player_id == player.id {
                    "in una"
                } else {
                    "nella tua"
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
            Lang::En => format!("You don't have enough balance to buy \"{}\"", pornstar.name),
            Lang::It => format!(
                "Non hai abbastanza soldi per comprare \"{}\"",
                pornstar.name
            ),
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
                    Lang::En => format!("Pornstar \"{}\" now is in your team", pornstar.name),
                    Lang::It => format!(
                        "Il/la pornostar \"{}\" ora è nella tua squadra",
                        pornstar.name
                    ),
                },
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
