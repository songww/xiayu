use std::convert::TryFrom;

mod value;

pub use value::MyValue;

#[cfg(feature = "mysql")]
impl<'a> super::HasVisitor<'a> for sqlx::MySql {
    type Visitor = crate::visitors::Mysql<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

macro_rules! bind {
    ($query:ident, $value: ident) => {
        match $value {
            MyValue::I8(int8) => $query.bind(int8),
            MyValue::I16(int16) => $query.bind(int16),
            MyValue::I32(int32) => $query.bind(int32),
            MyValue::I64(int64) => $query.bind(int64),
            MyValue::Float(float) => $query.bind(float) ,
            MyValue::Double(double) => $query.bind(double),
            MyValue::Boolean(boolean) => $query.bind(boolean),
            MyValue::Text(text) => $query.bind(text.map(|text|text.into_owned())),
            MyValue::Bytes(bytes) => $query.bind(bytes.map(|b|b.into_owned())),
            MyValue::U8(uint8) => $query.bind(uint8),
            MyValue::U16(uint16) => $query.bind(uint16),
            MyValue::U32(uint32) => $query.bind(uint32),
            MyValue::U64(uint64) => $query.bind(uint64),
            #[cfg(feature = "json")]
            MyValue::Json(json) => $query.bind(json),
            #[cfg(feature = "uuid")]
            MyValue::Uuid(uuid) => $query.bind(uuid),
            #[cfg(feature = "bigdecimal")]
            MyValue::BigDecimal(bigdecimal) => $query.bind(bigdecimal),
            #[cfg(feature = "decimal")]
            MyValue::Decimal(decimal) => $query.bind(decimal),
            #[cfg(feature = "chrono")]
            MyValue::UtcDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            MyValue::LocalDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            MyValue::NaiveDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            MyValue::NaiveDate(date) => $query.bind(date),
            #[cfg(feature = "chrono")]
            MyValue::NaiveTime(time) => $query.bind(time),
        }
    }
}


#[cfg(feature = "mysql")]
impl<'a> super::TryBind<'a, sqlx::MySql>
    for sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = MyValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}

#[cfg(feature = "mysql")]
impl<'a, O> super::TryBind<'a, sqlx::MySql>
    for sqlx::query::QueryAs<'a, sqlx::MySql, O, sqlx::mysql::MySqlArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = MyValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}
