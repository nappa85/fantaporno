use chrono::{NaiveDateTime, Utc};
use sea_orm::{entity::prelude::*, ActiveValue};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "chats")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub start_date: NaiveDateTime,
    pub lang: Lang,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::player::Entity")]
    Player,
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Player.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(2))")]
pub enum Lang {
    #[sea_orm(string_value = "en")]
    En,
    #[sea_orm(string_value = "it")]
    It,
}

impl TryFrom<&str> for Lang {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "en" => Ok(Lang::En),
            "it" => Ok(Lang::It),
            _ => Err(()),
        }
    }
}

pub async fn find_or_insert<C: ConnectionTrait>(
    conn: &C,
    id: impl Into<i64>,
) -> Result<Model, DbErr> {
    let id = id.into();
    let chat = Entity::find_by_id(id).one(conn).await?;
    if let Some(c) = chat {
        return Ok(c);
    }

    ActiveModel {
        id: ActiveValue::Set(id),
        start_date: ActiveValue::Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(conn)
    .await
}
