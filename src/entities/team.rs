use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "teams")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub player_id: i64,
    pub pornstar_id: i64,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::player::Entity")]
    Player,
    #[sea_orm(has_one = "super::pornstar::Entity")]
    Pornstar,
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Player.def()
    }
}

impl Related<super::pornstar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Pornstar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// #[cfg(test)]
// pub mod tests {
//     use chrono::DateTime;

//     pub fn mock_team() -> [super::Model; 1] {
//         [super::Model {
//             player_id: 1,
//             pornstar_id: 1,
//             start_date: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
//             end_date: None,
//         }]
//     }
// }
