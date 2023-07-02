use sea_orm::{entity::prelude::*, ActiveValue};

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
}

impl Related<super::position::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Position.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

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
