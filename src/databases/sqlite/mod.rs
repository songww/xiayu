use std::convert::TryFrom;

mod value;

pub use value::SQLiteValue;

#[cfg(feature = "sqlite")]
impl<'a> super::HasVisitor<'a> for sqlx::Sqlite {
    type Visitor = crate::visitors::Sqlite<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

/*
impl super::HasValue for sqlx::Sqlite {
    type Value = SQLiteValue;
}
*/


macro_rules! bind {
    ($query:ident, $value: ident) => {
        match $value {
            SQLiteValue::I8(int8) => $query.bind(int8),
            SQLiteValue::I16(int16) => $query.bind(int16),
            SQLiteValue::I32(int32) => $query.bind(int32),
            SQLiteValue::I64(int64) => $query.bind(int64),
            SQLiteValue::Float(float) => $query.bind(float) ,
            SQLiteValue::Double(double) => $query.bind(double),
            SQLiteValue::Boolean(boolean) => $query.bind(boolean),
            SQLiteValue::Text(text) => $query.bind(text.map(|text|text.into_owned())),
            SQLiteValue::Bytes(bytes) => $query.bind(bytes.map(|b|b.into_owned())),
            SQLiteValue::U8(uint8) => $query.bind(uint8),
            SQLiteValue::U16(uint16) => $query.bind(uint16),
            SQLiteValue::U32(uint32) => $query.bind(uint32),
            #[cfg(feature = "json")]
            SQLiteValue::Json(json) => $query.bind(json),
            #[cfg(feature = "uuid")]
            SQLiteValue::Uuid(uuid) => $query.bind(uuid),
            #[cfg(feature = "chrono")]
            SQLiteValue::UtcDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            SQLiteValue::LocalDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            SQLiteValue::NaiveDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            SQLiteValue::NaiveDate(date) => $query.bind(date),
            #[cfg(feature = "chrono")]
            SQLiteValue::NaiveTime(time) => $query.bind(time),
        }
    }

}

#[cfg(feature = "sqlite")]
impl<'a> super::TryBind<'a, sqlx::Sqlite>
    for sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = SQLiteValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}

#[cfg(feature = "sqlite")]
impl<'a, O> super::TryBind<'a, sqlx::Sqlite>
    for sqlx::query::QueryAs<'a, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'a>>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = SQLiteValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}
