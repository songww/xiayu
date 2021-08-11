#[macro_use]
extern crate xiayu_derive;

use std::marker::PhantomData;

use xiayu::prelude::Entity;
use xiayu_derive::*;

#[derive(Debug)]
pub struct Relation<T> {
    _phantom: PhantomData<T>,
}

impl<T> Relation<T> {
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Entity)]
#[tablename = "entities"]
pub struct AnEntity {
    #[column(primary_key, autoincrement, comment = "some comments")]
    pub id: i32,
    pub another_entity_id: Relation<AnotherEntity>,
    pub maybe_float: Option<f32>,
}

#[derive(Debug, Entity)]
pub struct AnotherEntity {
    pub id: i32,
}

#[test]
fn another_entity_definitions() {
    let entity = AnotherEntity { id: 1 };
    assert_eq!(entity.id, 1);
    assert_eq!(AnotherEntity::tablename(), "another_entities");
    assert_eq!(entity.tablename(), "another_entities");
}

#[test]
fn entity_definitions() {
    let entity = AnEntity {
        id: 2,
        another_entity_id: Relation::<AnotherEntity>::new(),
        maybe_float: None,
    };
    assert_eq!(entity.id, 2);
    assert_eq!(AnEntity::tablename(), "entities");
    assert_eq!(entity.tablename(), "entities");
}
