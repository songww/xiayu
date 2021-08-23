use std::marker::PhantomData;

use xiayu::prelude::*;

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
    #[column(primary_key)]
    pub id: i32,
}

#[test]
fn another_entity_definitions() {
    let entity = AnotherEntity { id: 1 };
    assert_eq!(entity.id, 1);
    assert_eq!(<AnotherEntity as Entity>::tablename(), "another_entities");
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
    assert_eq!(<AnEntity as Entity>::tablename(), "entities");
    assert_eq!(entity.tablename(), "entities");

    if cfg!(feature = "sqlite") {
        use sqlx::Connection;
        use sqlx::Executor;
        async fn run() -> Result<()> {
            let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:").await?;
            conn.execute("
                CREATE TABLE IF NOT EXISTS another_entities (
	                id INTEGER PRIMARY KEY
                );").await?;
            conn.execute("INSERT INTO another_entities (id) VALUES (1), (2);").await?;
            let mut entity = AnotherEntity::get(1).conn(&mut conn).await?;
            assert_eq!(entity.id, 1);
            entity.delete().conn(&mut conn).await?;
            match AnotherEntity::get(1).conn(&mut conn).await {
                Err(err) =>  {
                    match err.kind() {
                        xiayu::error::ErrorKind::NotFound(_) => {}
                        _ => return Err(err)
                    }
                }
                Ok(_) => panic!("Delete failed.")
            }
            let entity = AnotherEntity::get(2).conn(&mut conn).await?;
            assert_eq!(entity.id, 2);
            Ok(())
        }
        let res = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run());
        assert!(res.is_ok(), "{:?}", res)
    }
}
