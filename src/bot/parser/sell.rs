use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
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
    let Some(team) = crate::entities::team::Entity::find_by_id((player.id, pornstar.id))
        .filter(
            crate::entities::team::Column::EndDate
                .is_null()
                .or(crate::entities::team::Column::EndDate.gt(now)),
        )
        .one(conn)
        .await?
    else {
        return Ok(Err(match chat.lang {
            Lang::En => format!("Pornstar \"{}\" isn't in your team", pornstar.link()),
            Lang::It => format!(
                "Il/la pornostar \"{}\" non è nella tua squadra",
                pornstar.link()
            ),
        }));
    };

    let cost = match pornstar.get_cost(conn, chat.lang).await? {
        Ok(cost) => cost,
        Err(err) => return Ok(Err(err)),
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
                chat.id,
                match chat.lang {
                    Lang::En => format!("Pornstar \"{}\" is now free", pornstar.link()),
                    Lang::It => format!("Il/la pornostar \"{}\" è ora libero/a", pornstar.link()),
                },
            )
            .with_parse_mode(ParseMode::Markdown)
            .with_reply_parameters(ReplyParameters::new(message_id)),
        )
        .await?;

    Ok(Ok(()))
}
