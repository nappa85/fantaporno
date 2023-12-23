use chrono::NaiveDateTime;
use futures_util::stream::TryStreamExt;
use sea_orm::{entity::prelude::*, ActiveValue, Condition, QueryOrder, StreamTrait};
use std::{collections::HashMap, future};

use super::chat::Lang;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub telegram_id: i64,
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
    /// recalculate player's score based on entire player history
    pub async fn score<C: ConnectionTrait + StreamTrait>(
        &self,
        conn: &C,
        date: NaiveDateTime,
    ) -> Result<i64, DbErr> {
        let teams = super::team::Entity::find()
            .filter(
                super::team::Column::PlayerId.eq(self.id).and(
                    super::team::Column::EndDate
                        .is_null()
                        .or(super::team::Column::EndDate.gt(date)),
                ),
            )
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
            return Ok(0);
        }

        let positions = super::position::Entity::find()
            .filter(filter)
            .order_by_asc(super::position::Column::Date)
            .stream(conn)
            .await?;

        let pornstars = positions
            .try_fold(HashMap::new(), |mut pornstars, position| {
                let pornstar: &mut Vec<i32> = pornstars.entry(position.pornstar_id).or_default();
                pornstar.push(position.position);
                future::ready(Ok(pornstars))
            })
            .await?;

        Ok(pornstars
            .values()
            .map(|positions| {
                positions
                    .windows(2)
                    .map(|window| i64::from(window[0]) - i64::from(window[1]))
                    .sum::<i64>()
            })
            .sum::<i64>())
    }
}

pub async fn insert<C: ConnectionTrait>(
    conn: &C,
    telegram_id: impl Into<i64>,
    chat_id: i64,
    name: String,
) -> Result<Model, DbErr> {
    ActiveModel {
        telegram_id: ActiveValue::Set(telegram_id.into()),
        chat_id: ActiveValue::Set(chat_id),
        name: ActiveValue::Set(name),
        budget: ActiveValue::Set(super::BUDGET),
        ..Default::default()
    }
    .insert(conn)
    .await
}

pub async fn find<C: ConnectionTrait>(
    conn: &C,
    user_id: impl Into<i64>,
    chat_id: i64,
    lang: Lang,
) -> Result<Result<Model, String>, DbErr> {
    let Some(player) = crate::entities::player::Entity::find()
        .filter(
            crate::entities::player::Column::TelegramId
                .eq(user_id.into())
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

    Ok(Ok(player))
}

#[cfg(test)]
pub mod tests {
    use chrono::Utc;
    use sea_orm::{DbBackend, EntityTrait, MockDatabase};

    pub fn mock_player() -> [super::Model; 1] {
        [super::Model {
            id: 1,
            telegram_id: 1,
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
