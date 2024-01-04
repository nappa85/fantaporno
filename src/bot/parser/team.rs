use std::future;

use chrono::Utc;
use futures_util::TryStreamExt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, StreamTrait,
};
use tgbot::{
    api::Client,
    types::{ReplyParameters, SendMessage, User},
};

use crate::Error;

use super::{Chat, Lang};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    reply_to_message_id: i64,
    chat: &Chat,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + StreamTrait,
{
    let player = match crate::entities::player::find(conn, user.id, chat.id, chat.lang).await? {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    let now = Utc::now().naive_utc();
    let team = crate::entities::team::Entity::find()
        .filter(
            crate::entities::team::Column::PlayerId.eq(player.id).and(
                crate::entities::team::Column::EndDate
                    .is_null()
                    .or(crate::entities::team::Column::EndDate.lt(now)),
            ),
        )
        .select_only()
        .column(crate::entities::team::Column::PornstarId)
        .into_tuple::<i32>()
        .all(conn)
        .await?;

    let mut costs = crate::entities::pornstar::get_costs(conn, team.clone())
        .await?
        .unwrap_or_default();

    let list = crate::entities::pornstar::Entity::find()
        .filter(crate::entities::pornstar::Column::Id.is_in(team))
        .order_by_asc(crate::entities::pornstar::Column::Name)
        .stream(conn)
        .await?
        .try_fold(String::new(), move |mut buf, pornstar| {
            buf.push('\n');
            buf.push_str(&pornstar.name);
            if let Some(cost) = costs.remove(&pornstar.id) {
                buf.push_str(" (");
                buf.push_str(&cost.to_string());
                buf.push_str("€)");
            }
            future::ready(Ok(buf))
        })
        .await?;

    client
        .execute(
            SendMessage::new(
                chat.id,
                if list.is_empty() {
                    String::from(match chat.lang {
                        Lang::En => "Your team is empty",
                        Lang::It => "La tua squadra è vuota",
                    })
                } else {
                    match chat.lang {
                        Lang::En => format!("Your team is:{}", list),
                        Lang::It => format!("La tua squadra è:{}", list),
                    }
                },
            )
            .with_reply_parameters(ReplyParameters::new(reply_to_message_id)),
        )
        .await?;

    Ok(Ok(()))
}
