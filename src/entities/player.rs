use std::collections::HashMap;

use chrono::NaiveDateTime;
use futures_util::stream::TryStreamExt;
use sea_orm::{entity::prelude::*, ActiveValue, Statement, StreamTrait};
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
        pornstar_ids: Option<I>,
    ) -> Result<HashMap<i32, History>, DbErr>
    where
        C: ConnectionTrait + StreamTrait,
        I: IntoIterator<Item = i32>,
        <I as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        // fucking ORM making complex queries a nightmare
        let mut query = String::from("SELECT t.start_date, p.pornstar_id, p.date, p.position FROM teams t
        INNER JOIN positions p ON p.pornstar_id = t.pornstar_id AND p.date >= t.start_date AND (t.end_date IS NULL OR p.date <= t.end_date)
        WHERE t.player_id = ?");
        let mut params = vec![Value::from(self.id)];
        if let Some(pornstar_ids) = pornstar_ids {
            let iter = pornstar_ids.into_iter();
            query.push_str(&format!(
                " AND t.pornstar_id IN ({})",
                vec!["?"; iter.len()].join(", ")
            ));
            params.extend(iter.map(Value::from));
        }

        let stmt = Statement::from_sql_and_values(conn.get_database_backend(), query, params);
        conn.stream(stmt)
            .await?
            .try_fold(HashMap::new(), |mut pornstars, row| async move {
                let (start_date, pornstar_id, date, position) = row.try_get_many_by_index()?;
                let pornstar: &mut History = pornstars.entry(pornstar_id).or_default();
                pornstar.push(start_date, date, position);
                Ok(pornstars)
            })
            .await
    }

    /// recalculate player's score based on entire player history
    pub async fn score<C: ConnectionTrait + StreamTrait>(&self, conn: &C) -> Result<i32, DbErr> {
        let pornstars = self.history(conn, None::<[i32; 0]>).await?;

        Ok(pornstars.values().map(History::score).sum::<i32>())
    }
}

#[derive(Debug, Default)]
pub struct History(Vec<(NaiveDateTime, NaiveDateTime, i32)>);

impl History {
    fn push(&mut self, start_date: NaiveDateTime, date: NaiveDateTime, position: i32) {
        self.0.push((start_date, date, position));
    }

    pub fn scores(&self) -> impl DoubleEndedIterator<Item = (NaiveDateTime, i32)> + '_ {
        self.0.windows(2).filter_map(|window| {
            let (start_date0, _, position0) = window[0];
            let (start_date1, date, position1) = window[1];
            (start_date0 == start_date1).then_some((date, position0 - position1))
        })
    }

    pub fn score(&self) -> i32 {
        self.scores().map(|(_, i)| i).sum::<i32>()
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

    // #[tokio::test]
    // async fn score() {
    //     // queries must be in the order they are executed
    //     let conn: sea_orm::DatabaseConnection = MockDatabase::new(DbBackend::Sqlite)
    //         .append_query_results([mock_player()])
    //         .append_query_results([crate::entities::team::tests::mock_team()])
    //         .append_query_results([crate::entities::position::tests::mock_positions()])
    //         .into_connection();

    //     let player = super::Entity::find_by_id(1)
    //         .one(&conn)
    //         .await
    //         .unwrap()
    //         .unwrap();
    //     let score = player.score(&conn).await.unwrap();
    //     assert_eq!(score, 9);
    // }

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
        let score = player.score(&conn).await.unwrap();
        assert_eq!(score, 0);
    }
}
