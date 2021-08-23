use std::marker::{PhantomData, PhantomPinned};

use sqlx::types::chrono;
use sqlx::{Arguments, Database, IntoArguments, FromRow};

use crate::ast::Value;
use crate::prelude::{Delete, Entity, HasPrimaryKey, Select};
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

pub struct Values<'a>(Vec<crate::ast::Value<'a>>);

macro_rules! impl_into_arguments_for {
    ($arguments: path, $database: path) => {
        impl<'a> sqlx::IntoArguments<'a, $database> for Values<'a> 
        {
            fn into_arguments(self) -> $arguments {
                let mut args = <$arguments>::default();

                macro_rules! args_add {
                    ($v:expr, $ty: ty) => {
                        match $v {
                            Some(v) => args.add::<$ty>(v),
                            None => args.add::<Option<$ty>>(None),
                        }
                    };
                }

                macro_rules! to_json_value {
                    ($v:expr) => {
                        match $v {
                            Value::Array(Some(array)) => serde_json::Value::Array(
                                array.into_iter().map(|v| match v {
                                    Value::Array(_) => {
                                        panic!("Nested of nested of nested array are not supported yet.")
                                    },
                                    _ => {
                                        serde_json::Value::from(v.clone())
                                    }
                                }).collect::<Vec<_>>(),
                            ),
                            Value::Array(None) => serde_json::Value::Null,
                            _ => serde_json::Value::from($v.clone()),
                        }
                    };
                }

                macro_rules! value_to_argument {
                    ($value:expr) => {
                        match $value {
                            Value::Boolean(boolean) => args_add!(boolean, bool),
                            Value::Integer(integer) => args_add!(integer, i64),
                            Value::Float(float) => args_add!(float, f32),
                            Value::Double(double) => args_add!(double, f64),
                            Value::Text(text) => args_add!(text.clone().map(|v| v.to_string()), String),
                            Value::Enum(enumerable) => args_add!(enumerable.clone().map(|v| v.to_string()), String),
                            Value::Bytes(bytes) => {
                                args_add!(bytes.clone().map(|ref v| v.to_vec()), Vec<u8>)
                            }
                            Value::Char(char_) => args_add!(char_.map(|v| v.to_string()), String),
                            #[cfg(all(feature = "postgres", feature = "json-type"))]
                            Value::Array(array) => args_add!(
                                array.clone().map(|array| {
                                        array.into_iter()
                                        .map(|v| to_json_value!(v))
                                        .collect::<Vec<_>>()
                                }),
                                Vec<serde_json::Value>
                            ),
                            #[cfg(feature = "sqlite")]
                            Value::Array(array) => unimplemented!("Arrays are not supported in SQLite."),
                            #[cfg(feature = "bigdecimal-type")]
                            Value::Numeric(numeric) => args_add!(numeric, bigdecimal::BigDecimal),
                            #[cfg(feature = "json-type")]
                            Value::Json(json) => args_add!(json.clone(), serde_json::value::Value),
                            Value::Xml(xml) => args_add!(xml.clone().map(|v| v.to_string()), String),
                            #[cfg(feature = "uuid-type")]
                            Value::Uuid(uuid) => args_add!(uuid.clone(), sqlx::types::Uuid),
                            #[cfg(feature = "chrono-type")]
                            Value::DateTime(datetime) => {
                                args_add!(datetime.clone(), chrono::DateTime<chrono::Utc>)
                            }
                            #[cfg(feature = "chrono-type")]
                            Value::Date(date) => args_add!(date.clone(), chrono::NaiveDate),
                            #[cfg(feature = "chrono-type")]
                            Value::Time(time) => args_add!(time.clone(), chrono::NaiveTime),
                        }
                    };
                }

                for value in self.0.into_iter() {
                    value_to_argument!(value);
                }
                args
            }
        }
    };
}

#[cfg(feature = "postgres")]
impl_into_arguments_for!(::sqlx::postgres::PgArguments, ::sqlx::postgres::Postgres);

#[cfg(feature = "mssql")]
impl_into_arguments_for!(::sqlx::postgres::MssqlArguments<'a>, ::sqlx::postgres::Mssql);

#[cfg(feature = "mysql")]
impl_into_arguments_for!(::sqlx::mysql::MySqlArguments<'a>, ::sqlx::mysql::MySql);

#[cfg(feature = "sqlite")]
impl_into_arguments_for!(::sqlx::sqlite::SqliteArguments<'a>, ::sqlx::sqlite::Sqlite);

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
        Values<'a>: IntoArguments<'a, DB>
    {
        let (query, parameters) =
            <<C as sqlx::Executor<'a>>::Database as HasVisitor>::Visitor::build(self.select.clone())?;
        // 'a for borrowed from self.query
        self.compiled.replace(query);
        let arguments = IntoArguments::<'a, DB>::into_arguments(Values(parameters));
        let v = sqlx::query_as_with::<DB, T, _>(self.compiled.as_ref().unwrap(), arguments)
            .fetch_one(conn)
            .await?;
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
        Values<'a>: IntoArguments<'a, DB>
    {
        let (compiled, parameters) =
            <<C as sqlx::Executor<'a>>::Database as HasVisitor>::Visitor::build(self.delete.clone())?;
        // 'a for borrowed from self.compiled
        self.compiled.replace(compiled);
        let arguments = IntoArguments::<'a, DB>::into_arguments(Values(parameters));
        let _query_result = sqlx::query_with::<DB, _>(self.compiled.as_ref().unwrap(), arguments)
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
