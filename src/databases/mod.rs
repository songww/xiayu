use std::marker::{PhantomData};

#[cfg(feature = "chrono")]
use sqlx::types::chrono;
use sqlx::{Executor, Arguments, Database, IntoArguments, FromRow};
use async_trait::async_trait;

use crate::ast::{Value};
#[cfg(feature = "json")]
use crate::ast::Json;
use crate::prelude::{Delete, Entity, HasPrimaryKey, Select, Update};
use crate::visitors::Visitor;

pub trait HasVisitor<'a> {
    type Visitor: crate::visitors::Visitor<'a>;
    fn visitor() -> Self::Visitor;
}

#[cfg(feature = "postgres")]
impl<'a> HasVisitor<'a> for sqlx::Postgres {
    type Visitor = crate::visitors::Postgres<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "mssql")]
impl<'a> HasVisitor<'a> for sqlx::Mssql {
    type Visitor = crate::visitors::Mssql<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "mysql")]
impl<'a> HasVisitor<'a> for sqlx::MySql {
    type Visitor = crate::visitors::Mysql<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "sqlite")]
impl<'a> HasVisitor<'a> for sqlx::Sqlite {
    type Visitor = crate::visitors::Sqlite<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

// pub struct Values<'a>(Vec<crate::ast::Value<'a>>);

macro_rules! bind_value {
    ($query:ident, $value: ident) => {
        match $value {
            Value::Integer(integer) => $query.bind(integer),

            Value::I8(int8) => $query.bind(int8),
            Value::I16(int16) => $query.bind(int16),
            Value::I32(int32) => $query.bind(int32),
            Value::I64(int64) => $query.bind(int64),
            Value::Float(float) => $query.bind(float) ,
            Value::Double(double) => $query.bind(double),
            Value::Text(text) => $query.bind(text.map(|text|text.into_owned())),
            Value::Bytes(bytes) => $query.bind(bytes.map(|b|b.into_owned())),
            Value::Boolean(boolean) => $query.bind(boolean),
            #[cfg(feature = "json")]
            Value::Json(json) => match json {
                // #[cfg(feature = "postgres")]
                // Json::Json(json) => {
                //     //
                // }
                Json::JsonValue(value) => $query.bind(value),
                Json::JsonRawValue(raw_value) => $query.bind(raw_value.map(|v|sqlx::types::Json(*v)))
            },
            #[cfg(all(feature = "uuid", feature = "postgres"))]
            Value::Uuid(uuid) => $query.bind(uuid),
            #[cfg(feature = "postgres")]
            Value::PgInterval(interval) => $query.bind(interval),
            // #[cfg(feature = "postgres")]
            // Value::PgRange(range) => $query.bind(range),
            #[cfg(feature = "postgres")]
            Value::PgMoney(money) => $query.bind(money),
            #[cfg(all(feature = "bigdecimal", feature = "postgres"))]
            Value::BigDecimal(bigdecimal) => $query.bind(bigdecimal),
            #[cfg(all(feature = "decimal", feature = "postgres"))]
            Value::Decimal(decimal) => $query.bind(decimal),
            #[cfg(all(feature = "chrono", feature = "postgres"))]
            Value::UtcDateTime(datetime) => $query.bind(datetime),
            #[cfg(all(feature = "chrono", feature = "postgres"))]
            Value::LocalDateTime(datetime) => $query.bind(datetime),
            #[cfg(all(feature = "chrono", feature = "postgres"))]
            Value::NaiveDateTime(datetime) => $query.bind(datetime),
            #[cfg(all(feature = "chrono", feature = "postgres"))]
            Value::NaiveDate(date) => $query.bind(date),
            #[cfg(all(feature = "chrono", feature = "postgres"))]
            Value::NaiveTime(time) => $query.bind(time),
            #[cfg(all(feature = "time", feature = "postgres"))]
            Value::PgTimeTz(timetz) => $query.bind(timetz),
            #[cfg(all(feature = "ipnetwork", feature = "postgres"))]
            Value::IpNetwork(ipnetwork) => $query.bind(ipnetwork),

            _ => unimplemented!()
        }
    };
}

pub trait Binder<'a, DB> where DB: sqlx::Database {
    fn bind_value(self, value: Value<'a>) -> Self
    where
        Self: Sized;
}

#[cfg(feature = "postgres")]
impl<'a> Binder<'a, sqlx::Postgres> for sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "postgres")]
impl<'a, O> Binder<'a, sqlx::Postgres> for sqlx::query::QueryAs<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "mysql")]
impl<'a> Binder<'a, sqlx::MySql> for sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "mysql")]
impl<'a, O> Binder<'a, sqlx::MySql> for sqlx::query::QueryAs<'a, sqlx::MySql, O, sqlx::mysql::MySqlArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "mssql")]
impl<'a> Binder<'a, sqlx::Mssql> for sqlx::query::Query<'a, sqlx::Mssql, O, sqlx::mssql::MssqlArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "mssql")]
impl<'a, O> Binder<'a, sqlx::Mssql> for sqlx::query::QueryAs<'a, sqlx::Mssql, O, sqlx::mssql::MssqlArguments> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "sqlite")]
impl<'a> Binder<'a, sqlx::Sqlite> for sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

#[cfg(feature = "sqlite")]
impl<'a, O> Binder<'a, sqlx::Sqlite> for sqlx::query::QueryAs<'a, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'a>> {
    fn bind_value(self, value: Value<'a>) -> Self {
        bind_value!(self, value)
    }
}

/// fetch entity from table. Returned by [`get`][crate::prelude::HasPrimaryKey::get].
#[must_use = "query must be executed to affect database"]
pub struct FetchRequest<T, DB: Database> {
    select: Select<'static>,
    compiled: Option<String>,
    _marker: PhantomData<(T, DB)>,
}

impl<DB: Database, T: Send> FetchRequest<T, DB> {
    pub async fn conn<'a, C>(&'a mut self, conn: C) -> Result<T, crate::error::Error>
    where
        C: 'a + sqlx::Executor<'a, Database = DB>,
        DB: 'a + sqlx::Database + HasVisitor<'a>,
        <DB as sqlx::database::HasArguments<'a>>::Arguments: 'a + IntoArguments<'a, DB>,
        T: 'a + for<'r> sqlx::FromRow<'r, <DB as sqlx::Database>::Row> + Send + Unpin,
        sqlx::query::QueryAs<'a, DB, T, <DB as sqlx::database::HasArguments<'a>>::Arguments>: Binder<'a, DB>
    {
        let (query, parameters) =
            <<C as sqlx::Executor<'a>>::Database as HasVisitor>::Visitor::build(self.select.clone())?;
        // 'a for borrowed from self.query
        self.compiled.replace(query);
        let mut query = sqlx::query_as::<DB, T>(self.compiled.as_ref().unwrap());
        for parameter in parameters {
            query = query.bind_value(parameter);
        }

        let v = query.fetch_one(conn).await?;
        Ok(v)
    }
}

impl<T, DB> From<crate::ast::Select<'static>> for FetchRequest<T, DB>
where
    T: for<'r> FromRow<'r, <DB as Database>::Row>,
    DB: sqlx::Database
{
    fn from(select: crate::ast::Select<'static>) -> Self {
        Self {
            select,
            compiled: None,
            _marker: PhantomData,
        }
    }
}

/// fetch entity from table. Returned by [`get`][crate::prelude::HasPrimaryKey::get].
#[must_use = "delete must be executed to affect database"]
pub struct DeleteRequest<'a, E, DB> {
    delete: Delete<'static>,
    compiled: Option<String>,
    entity: &'a mut E,
    _marker: PhantomData<DB>,
}

impl<'e, E: HasPrimaryKey, DB: Database> DeleteRequest<'e, E, DB> {
    pub fn new<'a>(delete: crate::ast::Delete<'static>, entity: &'e mut E) -> Self
    {
        Self {
            entity,
            delete,
            compiled: None,
            _marker: PhantomData,
        }
    }

    pub async fn conn<'a, C>(&'a mut self, conn: C) -> Result<(), crate::error::Error>
    where
        'e: 'a,
        C: 'a + sqlx::Executor<'a, Database = DB>,
        DB: 'a + sqlx::Database + HasVisitor<'a>,
        <DB as sqlx::database::HasArguments<'a>>::Arguments: 'a + IntoArguments<'a, DB>,
        sqlx::query::Query<'a, DB, <DB as sqlx::database::HasArguments<'a>>::Arguments>: Binder<'a, DB>
    {
        let (compiled, parameters) =
            <<C as sqlx::Executor<'a>>::Database as HasVisitor>::Visitor::build(self.delete.clone())?;
        // 'a for borrowed from self.compiled
        self.compiled.replace(compiled);
        let mut query = sqlx::query::<DB>(self.compiled.as_ref().unwrap());
        for parameter in parameters {
            // query = bind_value!(query, parameter);
            query = query.bind_value(parameter);
        }

        let _query_result = query
            .execute(conn)
            .await?;
        Ok(())
    }
}

/// fetch entity from table. Returned by [`get`][crate::prelude::HasPrimaryKey::get].
#[must_use = "save must be executed to affect database"]
pub struct SaveRequest<'a, E, DB> {
    saving: Update<'static>,
    compiled: Option<String>,
    entity: &'a mut E,
    _marker: PhantomData<DB>,
}

impl<'e, E: HasPrimaryKey, DB: Database> SaveRequest<'e, E, DB> {
    pub fn new<'a>(saving: Update<'static>, entity: &'e mut E) -> Self
    {
        Self {
            entity,
            saving,
            compiled: None,
            _marker: PhantomData,
        }
    }

    #[must_use = "this must be used."]
    pub async fn conn<'a, C>(&'a mut self, conn: C) -> Result<(), crate::error::Error>
    where
        'e: 'a,
        C: 'a + Executioner<'a, DB>,
        DB: 'a + sqlx::Database + for<'v> HasVisitor<'v>,
        <DB as sqlx::database::HasArguments<'a>>::Arguments: 'a + IntoArguments<'a, DB>,
        sqlx::query::Query<'a, DB, <DB as sqlx::database::HasArguments<'a>>::Arguments>: Binder<'a, DB>
    {
        let (compiled, parameters) =
            <<C as sqlx::Executor<'a>>::Database as HasVisitor>::Visitor::build(self.saving.clone())?;
        // 'a for borrowed from self.compiled
        println!("compiled update: {}", &compiled);
        self.compiled.replace(compiled);
        let mut query = sqlx::query::<DB>(self.compiled.as_ref().unwrap());
        for parameter in parameters {
            query = query.bind_value(parameter);
        }
        let _query_result = query
            .execute(conn)
            .await?;
        Ok(())
    }
}


/// create table. Returned by [`get`][crate::prelude::entity::create_table].
#[must_use = "delete must be executed to affect database"]
pub struct CreateTable<DB> {
    _marker: PhantomData<DB>,
    compiled: Option<String>,
}

#[async_trait]
pub trait Executioner<'c, DB>: sqlx::Executor<'c, Database = DB> where DB: for<'v> HasVisitor<'v> + sqlx::Database {
    async fn save<E: HasPrimaryKey + Send>(self, entity: &mut E) -> crate::Result<()>;
}

macro_rules! impl_executioner_for {
    (<$($lifetime: lifetime),*>, $executor: ty, $database: ty) => {
        #[async_trait]
        impl<$($lifetime),*> Executioner<'c, $database> for $executor {
            async fn save<E: HasPrimaryKey + Send>(self, entity: &mut E) -> crate::Result<()> {
                let mut request = entity.save::<$database>();
                let (compiled, parameters) =
                    <$database as HasVisitor>::Visitor::build(request.saving.clone())?;
                // 'a for borrowed from self.compiled
                // println!("compiled saving: {}", &compiled);
                // println!("parameters ---> {:?}", parameters);
                request.compiled.replace(compiled);
                let mut query = sqlx::query::<$database>(request.compiled.as_ref().unwrap());
                for parameter in parameters {
                    query = query.bind_value(parameter);
                }
                // println!("query ---> {:?}", query);
                let _query_result = self.execute(query).await?;
                Ok(())
            }
        }
    };
}

#[cfg(feature = "mssql")]
impl_executioner_for!(<'c>, &'c mut sqlx::pool::PoolConnection<sqlx::Mssql>, sqlx::Mssql);
#[cfg(feature = "mysql")]
impl_executioner_for!(<'c>, &'c mut sqlx::pool::PoolConnection<sqlx::MySql>, sqlx::MySql);
#[cfg(feature = "sqlite")]
impl_executioner_for!(<'c>, &'c mut sqlx::pool::PoolConnection<sqlx::Sqlite>, sqlx::Sqlite);
#[cfg(feature = "postgres")]
impl_executioner_for!(<'c>, &'c mut sqlx::pool::PoolConnection<sqlx::Postgres>, sqlx::Postgres);
#[cfg(feature = "postgres")]
impl_executioner_for!(<'c>, &'c mut sqlx::postgres::PgListener, sqlx::Postgres);
#[cfg(feature = "mssql")]
impl_executioner_for!(<'c>, &'c mut sqlx::MssqlConnection, sqlx::Mssql);
#[cfg(feature = "mysql")]
impl_executioner_for!(<'c>, &'c mut sqlx::MySqlConnection, sqlx::MySql);
#[cfg(feature = "sqlite")]
impl_executioner_for!(<'c>, &'c mut sqlx::SqliteConnection, sqlx::Sqlite);
#[cfg(feature = "postgres")]
impl_executioner_for!(<'c>, &'c mut sqlx::PgConnection, sqlx::Postgres);
#[cfg(feature = "mssql")]
impl_executioner_for!(<'c, 't>, &'c mut sqlx::Transaction<'t, sqlx::Mssql>, sqlx::Mssql);
#[cfg(feature = "mysql")]
impl_executioner_for!(<'c, 't>, &'c mut sqlx::Transaction<'t, sqlx::MySql>, sqlx::MySql);
#[cfg(feature = "sqlite")]
impl_executioner_for!(<'c, 't>, &'c mut sqlx::Transaction<'t, sqlx::Sqlite>, sqlx::Sqlite);
#[cfg(feature = "postgres")]
impl_executioner_for!(<'c, 't>, &'c mut sqlx::Transaction<'t, sqlx::Postgres>, sqlx::Postgres);

#[async_trait]
impl<'p, DB> Executioner<'p, DB> for &'_ sqlx::Pool<DB> where
    DB: sqlx::Database + for <'v> HasVisitor<'v>,
    for<'c> &'c mut <DB as sqlx::Database>::Connection: Executioner<'c, DB>,
{
    async fn save<E: HasPrimaryKey + Send>(self, entity: &mut E) -> crate::Result<()> {
        let pool = self.clone();
        let mut conn = pool.acquire().await?;
        conn.save(entity).await
    }
}
