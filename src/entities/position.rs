use chrono::NaiveDateTime;
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
    date: NaiveDateTime,
    rank: i32,
) -> Result<bool, DbErr> {
    let position = Entity::find()
        .filter(Column::PornstarId.eq(pornstar_id))
        .order_by_desc(Column::Date)
        .one(conn)
        .await?;

    let inserted = if let Some(p) = position {
        match (p.date == date, p.position == rank) {
            (true, true) => return Ok(false),
            (true, false) => {
                ActiveModel {
                    pornstar_id: ActiveValue::Set(pornstar_id),
                    date: ActiveValue::Set(date),
                    ..Default::default()
                }
                .delete(conn)
                .await?;
                true
            }
            (false, diff) => !diff,
        }
    } else {
        true
    };

    ActiveModel {
        pornstar_id: ActiveValue::Set(pornstar_id),
        date: ActiveValue::Set(date),
        position: ActiveValue::Set(rank),
    }
    .insert(conn)
    .await?;

    Ok(inserted)
}

#[cfg(test)]
pub mod tests {
    use chrono::NaiveDateTime;

    pub fn mock_positions() -> [super::Model; 4] {
        [(1, 10), (2, 5), (3, 20), (4, 1)].map(|(timestamp, position)| super::Model {
            pornstar_id: 1,
            date: NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(),
            position,
        })
    }
}
