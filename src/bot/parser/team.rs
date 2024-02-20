use std::future;

use chrono::Utc;
use futures_util::TryStreamExt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, StreamTrait,
};
use tgbot::{
    api::Client,
    types::{ParseMode, ReplyParameters, SendMessage, User},
};

use crate::Error;

use super::{Chat, Lang, Tag};

pub async fn execute<C>(
    client: &Client,
    conn: &C,
    user: &User,
    tag: Option<Tag<'_>>,
    reply_to_message_id: i64,
    chat: &Chat,
) -> Result<Result<(), String>, Error>
where
    C: ConnectionTrait + StreamTrait,
{
    let res = if let Some(tag) = tag {
        tag.load_player(conn, chat.id, chat.lang).await?
    } else {
        crate::entities::player::find(conn, user, chat.id, chat.lang).await?
    };
    let player = match res {
        Ok(player) => player,
        Err(err) => return Ok(Err(err)),
    };

    let now = Utc::now().naive_utc();
    let team = crate::entities::team::Entity::find()
        .filter(
            crate::entities::team::Column::PlayerId.eq(player.id).and(
                crate::entities::team::Column::EndDate
                    .is_null()
                    .or(crate::entities::team::Column::EndDate.gt(now)),
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

    let mut history = player.history(conn, None::<[i32; 0]>).await?;

    let list = crate::entities::pornstar::Entity::find()
        .filter(crate::entities::pornstar::Column::Id.is_in(team))
        .order_by_asc(crate::entities::pornstar::Column::Name)
        .stream(conn)
        .await?
        .try_fold(String::new(), move |mut buf, pornstar| {
            buf.push('\n');
            buf.push_str(&pornstar.name);
            buf.push_str(" (");
            let cost = costs.remove(&pornstar.id).as_ref().map(ToString::to_string);
            let cost = cost.as_deref().unwrap_or("-");
            buf.push_str(cost);
            buf.push_str("€ | ");
            let history = history.remove(&pornstar.id).map(|positions| {
                let pos_mov = positions.score();
                if pos_mov >= 0 {
                    format!("+{pos_mov}")
                } else {
                    pos_mov.to_string()
                }
            });
            let history = history.as_deref().unwrap_or("-");
            buf.push_str(history);
            buf.push(')');
            future::ready(Ok(buf))
        })
        .await?;

    client
        .execute(
            SendMessage::new(
                chat.id,
                if player.telegram_id == i64::from(user.id) {
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
                    }
                } else if list.is_empty() {
                    match chat.lang {
                        Lang::En => format!("{}'s team is empty", player.tg_link()),
                        Lang::It => format!("La squadra di {} è vuota", player.tg_link()),
                    }
                } else {
                    match chat.lang {
                        Lang::En => format!("{}'s team is:{}", player.tg_link(), list),
                        Lang::It => format!("La squadra di {} è:{}", player.tg_link(), list),
                    }
                },
            )
            .with_parse_mode(ParseMode::Markdown)
            .with_reply_parameters(ReplyParameters::new(reply_to_message_id)),
        )
        .await?;

    Ok(Ok(()))
}
