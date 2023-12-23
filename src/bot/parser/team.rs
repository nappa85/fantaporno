use std::future;

use chrono::Utc;
use futures_util::TryStreamExt;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, StreamTrait};
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
        .all(conn)
        .await?;

    let list = crate::entities::pornstar::Entity::find()
        .filter(
            crate::entities::pornstar::Column::Id
                .is_in(team.into_iter().map(|team| team.pornstar_id)),
        )
        .order_by_asc(crate::entities::pornstar::Column::Name)
        .stream(conn)
        .await?
        .try_fold(String::new(), |mut buf, pornstar| {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(&pornstar.name);
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
                        Lang::En => format!("Your team is:\n{}", list),
                        Lang::It => format!("La tua squadra è:\n{}", list),
                    }
                },
            )
            .with_reply_to_message_id(reply_to_message_id),
        )
        .await?;

    Ok(Ok(()))
}
