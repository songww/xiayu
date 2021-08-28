#[macro_use]
extern crate xiayu_derive;

use xiayu::prelude::*;
use xiayu::visitors::Visitor;

#[derive(Debug, Entity)]
#[tablename = "entities"]
pub struct AnEntity {
    #[column(primary_key, autoincrement, comment = "some comments")]
    pub id: i32,
    pub another_entity_id: i32,
    pub maybe_float: Option<f32>,
}

#[derive(Debug, Entity)]
pub struct AnotherEntity {
    #[column(primary_key, autoincrement, comment = "some comments")]
    pub id: i32,
}

fn main() {
    let entity = AnotherEntity { id: 1 };
    assert_eq!(entity.id, 1);
    assert_eq!(<AnotherEntity as Entity>::tablename(), "another_entities");
    assert_eq!(entity.tablename(), "another_entities");

    let entity = AnEntity {
        id: 2,
        another_entity_id: 1,
        maybe_float: None,
    };
    assert_eq!(entity.id, 2);
    assert_eq!(<AnEntity as Entity>::tablename(), "entities");
    assert_eq!(entity.tablename(), "entities");

    #[cfg(feature = "postgres")]
    assert_eq!(
        xiayu::visitors::Postgres::build(
            Select::from_table(<AnEntity as Entity>::table())
                .so_that(AnEntity::another_entity_id.equals(1)),
        )
        .unwrap(),
        ("".to_string(), Vec::new())
    );
}
