use std::collections::HashMap;

use chrono::NaiveDateTime;
use sea_orm::{entity::prelude::*, ActiveValue, QuerySelect};

use super::chat::Lang;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pornstars")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub url: String,
}

impl Model {
    pub fn link(&self) -> String {
        format!("[{}](https://pornhub.com{})", self.name, self.url)
    }
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
        lang: Lang,
    ) -> Result<Result<u32, String>, crate::Error> {
        match get_costs(conn, [self.id])
            .await?
            .and_then(|mut costs| costs.remove(&self.id))
        {
            Some(cost) => Ok(Ok(cost)),
            None => Ok(Err(match lang {
                Lang::En => format!(
                    "Pornstar \"{}\" doesn't have a valuation at the moment",
                    self.name
                ),
                Lang::It => format!(
                    "Il/la pornostar \"{}\" non ha una valutazione in questo momento",
                    self.name
                ),
            })),
        }
    }
}

pub async fn get_costs<C: ConnectionTrait>(
    conn: &C,
    ids: impl IntoIterator<Item = i32>,
) -> Result<Option<HashMap<i32, u32>>, crate::Error> {
    let Some(max_date) = super::position::Entity::find()
        .select_only()
        .column_as(super::position::Column::Date.max(), "max")
        .into_tuple::<NaiveDateTime>()
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let positions = super::position::Entity::find()
        .filter(
            super::position::Column::PornstarId
                .is_in(ids)
                .and(super::position::Column::Date.eq(max_date)),
        )
        .all(conn)
        .await?;

    let Some(max_position) = super::position::Entity::find()
        .filter(super::position::Column::Date.eq(max_date))
        .select_only()
        .column_as(super::position::Column::Position.max(), "max")
        .into_tuple::<u32>()
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(
        positions
            .into_iter()
            .map(|position| {
                let super::position::Model {
                    pornstar_id,
                    position,
                    ..
                } = position;

                let Ok(position) = u32::try_from(position) else {
                    return Err(crate::Error::InvalidPosition);
                };

                Ok((pornstar_id, super::BUDGET * position / max_position))
            })
            .collect::<Result<_, _>>()?,
    ))
}

pub async fn find_or_insert<C: ConnectionTrait>(
    conn: &C,
    name: &str,
    url: &str,
) -> Result<Model, DbErr> {
    let pornstar = Entity::find().filter(Column::Url.eq(url)).one(conn).await?;
    if let Some(p) = pornstar {
        return if p.name == name {
            Ok(p)
        } else {
            ActiveModel {
                id: ActiveValue::Set(p.id),
                name: ActiveValue::Set(name.to_owned()),
                ..Default::default()
            }
            .update(conn)
            .await
        };
    }

    ActiveModel {
        name: ActiveValue::Set(name.to_owned()),
        url: ActiveValue::Set(url.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await
}

pub async fn search<C: ConnectionTrait>(
    conn: &C,
    name: &str,
    lang: Lang,
) -> Result<Result<Model, String>, DbErr> {
    if name.chars().count() < 3 {
        return Ok(Err(String::from(match lang {
            Lang::En => "search string too short",
            Lang::It => "la stringa di ricerca è troppo corta",
        })));
    }

    let mut pornstars = Entity::find()
        .filter(Column::Name.like(format!("%{name}%")))
        .all(conn)
        .await?;
    match pornstars.len() {
        0 => Ok(Err(match lang {
            Lang::En => format!("Pornstar \"{name}\" not found"),
            Lang::It => format!("Pornostar \"{name}\" non trovata/o"),
        })),
        1 => Ok(Ok(pornstars.remove(0))),
        _ => {
            if let Some(index) = pornstars
                .iter()
                .position(|pornstar| pornstar.name.eq_ignore_ascii_case(name))
            {
                Ok(Ok(pornstars.remove(index)))
            } else {
                Ok(Err(pornstars.into_iter().fold(
                    String::from(match lang {
                        Lang::En => "Which one do you mean?",
                        Lang::It => "Quale intendevi?",
                    }),
                    |mut buf, pornstar| {
                        buf.push_str("\n- `");
                        buf.push_str(&pornstar.name);
                        buf.push('`');
                        buf
                    },
                )))
            }
        }
    }
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
