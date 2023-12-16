use chrono::NaiveDateTime;
use futures_util::stream::TryStreamExt;
use sea_orm::{entity::prelude::*, Condition, QueryOrder, StreamTrait};
use std::{collections::HashMap, future};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub telegram_id: u32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::team::Entity")]
    Team,
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
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
                super::team::Column::PlayerId.eq(self.telegram_id).and(
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

// pub async fn find_or_insert<C: ConnectionTrait>(
//     conn: &C,
//     name: &str,
//     url: &str,
// ) -> Result<Model, DbErr> {
//     let pornstar = Entity::find()
//         .filter(Column::Name.eq(name).and(Column::Url.eq(url)))
//         .one(conn)
//         .await?;
//     if let Some(p) = pornstar {
//         return Ok(p);
//     }

//     ActiveModel {
//         name: ActiveValue::Set(name.to_owned()),
//         url: ActiveValue::Set(url.to_owned()),
//         ..Default::default()
//     }
//     .insert(conn)
//     .await
// }

#[cfg(test)]
pub mod tests {
    use chrono::Utc;
    use sea_orm::{DbBackend, EntityTrait, MockDatabase};

    pub fn mock_player() -> [super::Model; 1] {
        [super::Model {
            telegram_id: 1,
            name: String::from("pippo"),
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

        let player = super::Entity::find_by_id(1_u32)
            .one(&conn)
            .await
            .unwrap()
            .unwrap();
        let score = player.score(&conn, Utc::now().naive_utc()).await.unwrap();
        assert_eq!(score, 9);
    }
}
