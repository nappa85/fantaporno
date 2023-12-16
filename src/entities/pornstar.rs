use chrono::NaiveDateTime;
use sea_orm::{entity::prelude::*, ActiveValue, QuerySelect};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pornstars")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::position::Entity")]
    Position,
    #[sea_orm(has_many = "super::team::Entity")]
    Team,
}

impl Related<super::position::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Position.def()
    }
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub async fn get_cost<C: ConnectionTrait>(
        &self,
        conn: &C,
    ) -> Result<Option<u32>, crate::Error> {
        let Some(max_date) = super::position::Entity::find()
            .select_only()
            .column_as(super::position::Column::Date.max(), "max")
            .into_tuple::<NaiveDateTime>()
            .one(conn)
            .await?
        else {
            return Ok(None);
        };

        let Some(position) = super::position::Entity::find()
            .filter(
                super::position::Column::PornstarId
                    .eq(self.id)
                    .and(super::position::Column::Date.eq(max_date)),
            )
            .one(conn)
            .await?
        else {
            return Ok(None);
        };

        let Some(max_position) = super::position::Entity::find()
            .filter(super::position::Column::Date.eq(position.date))
            .select_only()
            .column_as(super::position::Column::Position.max(), "max")
            .into_tuple::<u32>()
            .one(conn)
            .await?
        else {
            return Ok(None);
        };

        let position =
            u32::try_from(position.position).map_err(|_| crate::Error::InvalidPosition)?;

        Ok(Some(super::BUDGET * position / max_position))
    }
}

pub async fn find_or_insert<C: ConnectionTrait>(
    conn: &C,
    name: &str,
    url: &str,
) -> Result<Model, DbErr> {
    let pornstar = Entity::find()
        .filter(Column::Name.eq(name).and(Column::Url.eq(url)))
        .one(conn)
        .await?;
    if let Some(p) = pornstar {
        return Ok(p);
    }

    ActiveModel {
        name: ActiveValue::Set(name.to_owned()),
        url: ActiveValue::Set(url.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await
}

// #[cfg(test)]
// pub mod tests {
//     pub fn mock_pornstar() -> [super::Model; 1] {
//         [super::Model {
//             id: 1,
//             name: String::from("Tua madre"),
//             url: String::from("lemonparty.com"),
//         }]
//     }
// }
