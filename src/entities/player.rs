use chrono::NaiveDateTime;
use futures_util::stream::TryStreamExt;
use sea_orm::{entity::prelude::*, ActiveValue, Condition, QueryOrder, StreamTrait};
use std::{collections::HashMap, future};
use tgbot::types::User;

use super::chat::Lang;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub telegram_id: i64,
    pub tag: Option<String>,
    pub chat_id: i64,
    pub name: String,
    pub budget: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::team::Entity")]
    Team,
    #[sea_orm(has_many = "super::chat::Entity")]
    Chat,
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::chat::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chat.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn tg_link(&self) -> String {
        format!("[{}](tg://user?id={})", self.name, self.telegram_id)
    }

    pub async fn history<C, I>(
        &self,
        conn: &C,
        date: NaiveDateTime,
        pornstar_ids: Option<I>,
    ) -> Result<Option<HashMap<i32, Vec<super::position::Model>>>, DbErr>
    where
        C: ConnectionTrait + StreamTrait,
        I: IntoIterator<Item = i32>,
    {
        let mut filter = super::team::Column::PlayerId
            .eq(self.id)
            .and(super::team::Column::StartDate.lte(date))
            .and(
                super::team::Column::EndDate
                    .is_null()
                    .or(super::team::Column::EndDate.gt(date)),
            );
        if let Some(pornstar_ids) = pornstar_ids {
            filter = filter.and(super::team::Column::PornstarId.is_in(pornstar_ids));
        }

        let teams = super::team::Entity::find()
            .filter(filter)
            .stream(conn)
            .await?;

        let filter = teams
            .try_fold(Condition::any(), |condition, team| {
                let cond = super::position::Column::PornstarId.eq(team.pornstar_id);
                let cond = if let Some(end_date) = team.end_date {
                    cond.and(super::position::Column::Date.between(team.start_date, end_date))
                } else {
                    cond.and(super::position::Column::Date.gte(team.start_date))
                };
                future::ready(Ok(condition.add(cond)))
            })
            .await?;
        if filter.is_empty() {
            return Ok(None);
        }

        let positions = super::position::Entity::find()
            .filter(filter)
            .order_by_asc(super::position::Column::Date)
            .stream(conn)
            .await?;

        Ok(Some(
            positions
                .try_fold(HashMap::new(), |mut pornstars, position| {
                    let pornstar: &mut Vec<super::position::Model> =
                        pornstars.entry(position.pornstar_id).or_default();
                    pornstar.push(position);
                    future::ready(Ok(pornstars))
                })
                .await?,
        ))
    }

    /// recalculate player's score based on entire player history
    pub async fn score<C: ConnectionTrait + StreamTrait>(
        &self,
        conn: &C,
        date: NaiveDateTime,
    ) -> Result<i64, DbErr> {
        let Some(pornstars) = self.history(conn, date, None::<[i32; 0]>).await? else {
            return Ok(0);
        };

        Ok(pornstars
            .values()
            .map(|positions| {
                positions
                    .windows(2)
                    .map(|window| i64::from(window[0].position) - i64::from(window[1].position))
                    .sum::<i64>()
            })
            .sum::<i64>())
    }
}

pub async fn insert<C: ConnectionTrait>(
    conn: &C,
    user: &User,
    chat_id: i64,
) -> Result<Model, DbErr> {
    ActiveModel {
        telegram_id: ActiveValue::Set(i64::from(user.id)),
        tag: ActiveValue::Set(user.username.clone().map(String::from)),
        chat_id: ActiveValue::Set(chat_id),
        name: ActiveValue::Set(get_name(user)),
        budget: ActiveValue::Set(super::BUDGET),
        ..Default::default()
    }
    .insert(conn)
    .await
}

pub async fn find<C: ConnectionTrait>(
    conn: &C,
    user: &User,
    chat_id: i64,
    lang: Lang,
) -> Result<Result<Model, String>, DbErr> {
    let Some(player) = crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(i64::from(user.id))
                .and(crate::entities::player::Column::ChatId.eq(chat_id)),
        )
        .one(conn)
        .await?
    else {
        return Ok(Err(String::from(match lang {
            Lang::En => "Player doesn't exists, use /start to create",
            Lang::It => "Giocatore inesistente, usa /start per crearlo",
        })));
    };

    let tag_changed = match (&player.tag, &user.username) {
        (Some(tag), Some(username)) => username != tag,
        (Some(_), None) | (None, Some(_)) => true,
        (None, None) => false,
    };
    let name_changed = if let Some(last_name) = &user.last_name {
        !player.name.starts_with(&user.first_name)
            || !player.name.ends_with(last_name)
            || player.name.len() != user.first_name.len() + last_name.len() + 1
    } else {
        player.name != user.first_name
    };
    if tag_changed || name_changed {
        ActiveModel {
            id: ActiveValue::Set(player.id),
            tag: ActiveValue::Set(user.username.clone().map(String::from)),
            name: ActiveValue::Set(get_name(user)),
            ..Default::default()
        }
        .update(conn)
        .await?;
    }

    Ok(Ok(player))
}

fn get_name(user: &User) -> String {
    if let Some(last_name) = &user.last_name {
        format!("{} {last_name}", user.first_name)
    } else {
        user.first_name.clone()
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::Utc;
    use sea_orm::{DbBackend, EntityTrait, MockDatabase};

    pub fn mock_player() -> [super::Model; 1] {
        [super::Model {
            id: 1,
            telegram_id: 1,
            tag: None,
            chat_id: 1,
            name: String::from("pippo"),
            budget: 0,
        }]
    }

    #[tokio::test]
    async fn score() {
        // queries must be in the order they are executed
        let conn: sea_orm::DatabaseConnection = MockDatabase::new(DbBackend::Sqlite)
            .append_query_results([mock_player()])
            .append_query_results([crate::entities::team::tests::mock_team()])
            .append_query_results([crate::entities::position::tests::mock_positions()])
            .into_connection();

        let player = super::Entity::find_by_id(1)
            .one(&conn)
            .await
            .unwrap()
            .unwrap();
        let score = player.score(&conn, Utc::now().naive_utc()).await.unwrap();
        assert_eq!(score, 9);
    }

    #[tokio::test]
    async fn empty_score() {
        // queries must be in the order they are executed
        let conn: sea_orm::DatabaseConnection = MockDatabase::new(DbBackend::Sqlite)
            .append_query_results([mock_player()])
            .append_query_results::<crate::entities::team::Model, _, _>([[]])
            .into_connection();

        let player = super::Entity::find_by_id(1)
            .one(&conn)
            .await
            .unwrap()
            .unwrap();
        let score = player.score(&conn, Utc::now().naive_utc()).await.unwrap();
        assert_eq!(score, 0);
    }
}
