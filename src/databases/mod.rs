use std::marker::{PhantomData, PhantomPinned};

use sqlx::types::chrono;
use sqlx::{Arguments};

use crate::ast::Value;

pub trait HasVisitor<'a> {
    type Visitor: crate::visitors::Visitor<'a>;
    fn visitor(&self) -> Self::Visitor;
}

#[cfg(feature = "postgres")]
impl<'a> HasVisitor<'a> for sqlx::Postgres {
    type Visitor = crate::visitors::Postgres<'a>;
    fn visitor(&self) -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "mssql")]
impl<'a> HasVisitor<'a> for sqlx::Mssql {
    type Visitor = crate::visitors::Mssql<'a>;
    fn visitor(&self) -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "mysql")]
impl<'a> HasVisitor<'a> for sqlx::MySql {
    type Visitor = crate::visitors::Mysql<'a>;
    fn visitor(&self) -> Self::Visitor {
        Self::Visitor::default()
    }
}

#[cfg(feature = "sqlite")]
impl<'a> HasVisitor<'a> for sqlx::Sqlite {
    type Visitor = crate::visitors::Sqlite<'a>;
    fn visitor(&self) -> Self::Visitor {
        Self::Visitor::default()
    }
}

pub trait ToArguments<'a, DB>
where
    DB: sqlx::Database,
{
    type TargetArguments: sqlx::Arguments<'a>;
    fn to_arguments(&self) -> Self::TargetArguments;
}

macro_rules! impl_to_arguments_for {
    ($arguments:ty) => {
        impl<'a> ToArguments<'a, sqlx::Postgres> for Vec<crate::ast::Value<'a>> {
            type TargetArguments = $arguments;
            fn to_arguments(&self) -> Self::TargetArguments {
                let mut args = Self::TargetArguments::default();

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
                            Value::Boolean(boolean) => args_add!(*boolean, bool),
                            Value::Integer(integer) => args_add!(*integer, i64),
                            Value::Float(float) => args_add!(*float, f32),
                            Value::Double(double) => args_add!(*double, f64),
                            Value::Text(text) => args_add!(text.clone().map(|v| v.to_string()), String),
                            Value::Enum(enumerable) => args_add!(enumerable.clone().map(|v| v.to_string()), String),
                            Value::Bytes(bytes) => {
                                args_add!(bytes.clone().map(|ref v| v.to_vec()), Vec<u8>)
                            }
                            Value::Char(char_) => args_add!(char_.map(|v| v.to_string()), String),
                            #[cfg(all(feature = "postgres", feature = "json-type"))]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "postgres")))]
                            Value::Array(array) => args_add!(
                                array.clone().map(|array| {
                                        array.into_iter()
                                        .map(|v| to_json_value!(v))
                                        .collect::<Vec<_>>()
                                }),
                                Vec<serde_json::Value>
                            ),
                            #[cfg(feature = "bigdecimal-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal-type")))]
                            Value::Numeric(numeric) => args_add!(numeric, bigdecimal::BigDecimal),
                            #[cfg(feature = "json-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "json-type")))]
                            Value::Json(json) => args_add!(json.clone(), serde_json::value::Value),
                            Value::Xml(xml) => args_add!(xml.clone().map(|v| v.to_string()), String),
                            #[cfg(feature = "uuid-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid-type")))]
                            Value::Uuid(uuid) => args_add!(uuid.clone(), sqlx::types::Uuid),
                            #[cfg(feature = "chrono-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono-type")))]
                            Value::DateTime(datetime) => {
                                args_add!(datetime.clone(), chrono::DateTime<chrono::Utc>)
                            }
                            #[cfg(feature = "chrono-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono-type")))]
                            Value::Date(date) => args_add!(date.clone(), chrono::NaiveDate),
                            #[cfg(feature = "chrono-type")]
                            #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono-type")))]
                            Value::Time(time) => args_add!(time.clone(), chrono::NaiveTime),
                        }
                    };
                }

                for value in self.into_iter() {
                    value_to_argument!(value);
                }
                args
            }
        }
    };
}

#[cfg(feature = "postgres")]
impl_to_arguments_for!(::sqlx::postgres::PgArguments);

#[cfg(feature = "mssql")]
impl_to_arguments_for!(::sqlx::postgres::MssqlArguments<'a>);

#[cfg(feature = "mysql")]
impl_to_arguments_for!(::sqlx::mysql::MySqlArguments<'a>);

#[cfg(feature = "sqlite")]
impl_to_arguments_for!(::sqlx::sqlite::SqliteArguments<'a>);

pub struct Executable<Output> {
    query: crate::ast::Query<'static>,
    many: bool,
    _marker: PhantomData<Output>
}

impl<Q, Output> From<(Q, bool)> for Executable<Output>
where
    Q: Into<crate::ast::Query<'static>>,
{
    fn from((query, many): (Q, bool)) -> Self {
        Self {
            query: query.into(),
            many,
            _marker: PhantomData,
        }
    }
}

/*
impl<Q, Output> From<Q> for Executable<Output>
where
    Q: Into<crate::ast::Query<'static>>,
{
    fn from(query: Q) -> Self {
        Self {
            query: query.into(),
            many: false,
            _marker: PhantomData,
        }
    }
}
*/

impl<Output> Executable<Output> {
    async fn execute<'a, E>(&self, db: E) -> Result<Output, sqlx::Error> where E: sqlx::Executor<'a> {
        match self {
            crate::ast::Query::Select(selecting) => {
                // Box<Select<'a>>
                if self.many {
                    db.fetch(inserting).await.map(|out| out.into())
                } else {
                    db.fetch_many(inserting).await.map(|out| out.into())
                }
            },
            crate::ast::Query::Insert(inserting) => {
                // Box<Insert<'a>>
                db.execute(inserting).await.map(|out| out.into())
            },
            crate::ast::Query::Update(updating) => {
                // Box<Update<'a>>
                db.execute(updating).await.map(|out| out.into())
            },
            crate::ast::Query::Delete(deleting) => {
                // Box<Delete<'a>>
                db.execute(unionist).await.map(|out| out.into())
            },
            crate::ast::Query::Union(unionist) => {
                // Box<Union<'a>>
                db.execute(unionist).await.map(|out| out.into())
            },
            crate::ast::Query::Merge(merging) => {
                // Box<Merge<'a>>
                db.execute(merging).await.map(|out| out.into())
            },
            crate::ast::Query::Raw(raw) => {
                db.execute(raw.as_ref()).await.map(|out| out.into())
            }
        }
    }
}
