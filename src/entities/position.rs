use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{entity::prelude::*, ActiveValue, QueryOrder};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "positions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub pornstar_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub date: NaiveDateTime,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::pornstar::Entity")]
    Pornstar,
}

impl Related<super::pornstar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Pornstar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn inserted<C: ConnectionTrait>(
    conn: &C,
    pornstar_id: i32,
    date: DateTime<Utc>,
    rank: i32,
) -> Result<bool, DbErr> {
    let position = Entity::find()
        .filter(Column::PornstarId.eq(pornstar_id))
        .order_by_desc(Column::Date)
        .one(conn)
        .await?;

    let inserted = if let Some(p) = position {
        rank != p.position
    } else {
        true
    };

    ActiveModel {
        pornstar_id: ActiveValue::Set(pornstar_id),
        date: ActiveValue::Set(date.naive_utc()),
        position: ActiveValue::Set(rank),
    }
    .insert(conn)
    .await?;

    Ok(inserted)
}
