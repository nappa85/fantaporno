use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use tgbot::{
    api::Client,
    types::{ChatPeerId, SendMessage, User},
};

use crate::Error;

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    message_id: i64,
    chat_id: ChatPeerId,
    pornstar_name: String,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + TransactionTrait,
{
    let Some(player) = crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(i64::from(user.id))
                .and(crate::entities::player::Column::ChatId.eq(i64::from(chat_id))),
        )
        .one(conn)
        .await?
    else {
        return Ok(Err("Player doesn't exists, use /start to create".into()));
    };

    let pornstar = match crate::entities::pornstar::search(conn, &pornstar_name).await {
        Ok(Ok(pornstar)) => pornstar,
        Ok(Err(err)) => return Ok(Err(err)),
        Err(err) => return Err(Error::from(err)),
    };

    let now = Utc::now().naive_utc();
    let Some(team) = crate::entities::team::Entity::find_by_id((player.id, pornstar.id))
        .filter(
            crate::entities::team::Column::EndDate
                .is_null()
                .or(crate::entities::team::Column::EndDate.gt(now)),
        )
        .one(conn)
        .await?
    else {
        return Ok(Err(format!(
            "Pornstar \"{}\" isn't in your team",
            pornstar.name,
        )));
    };

    let Some(cost) = pornstar.get_cost(conn).await? else {
        return Ok(Err(format!(
            "Pornstar \"{}\" doesn't have a valutation at the moment",
            pornstar.name
        )));
    };

    let txn = conn.begin().await?;

    let mut team = crate::entities::team::ActiveModel::from(team);
    team.end_date = ActiveValue::Set(Some(now));
    team.update(&txn).await?;

    crate::entities::player::ActiveModel {
        id: ActiveValue::Set(player.id),
        budget: ActiveValue::Set(player.budget + cost),
        ..Default::default()
    }
    .update(&txn)
    .await?;

    txn.commit().await?;

    client
        .execute(
            SendMessage::new(
                chat_id,
                format!("Pornstar \"{}\" is now free", pornstar.name),
            )
            .with_reply_to_message_id(message_id),
        )
        .await?;

    Ok(Ok(()))
}
