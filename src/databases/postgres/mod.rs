use std::convert::TryFrom;

mod value;

pub use value::PgValue;

#[cfg(feature = "postgres")]
impl<'a> super::HasVisitor<'a> for sqlx::Postgres {
    type Visitor = crate::visitors::Postgres<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

macro_rules! bind {
    ($query:ident, $value: ident) => {
        match $value {
            PgValue::I8(int8) => $query.bind(int8),
            PgValue::I16(int16) => $query.bind(int16),
            PgValue::I32(int32) => $query.bind(int32),
            PgValue::I64(int64) => $query.bind(int64),
            PgValue::Float(float) => $query.bind(float) ,
            PgValue::Double(double) => $query.bind(double),
            PgValue::Boolean(boolean) => $query.bind(boolean),
            PgValue::Text(text) => $query.bind(text.map(|text|text.into_owned())),
            PgValue::Bytes(bytes) => $query.bind(bytes.map(|b|b.into_owned())),
            PgValue::U32(uint32) => $query.bind(uint32),
            #[cfg(feature = "json")]
            PgValue::Json(json) => $query.bind(json),
            #[cfg(feature = "uuid")]
            PgValue::Uuid(uuid) => $query.bind(uuid),
            PgValue::PgInterval(interval) => $query.bind(interval),
            // #[cfg(feature = "postgres")]
            // PgValue::PgRange(range) => $query.bind(range),
            PgValue::PgMoney(money) => $query.bind(money),
            #[cfg(feature = "bigdecimal")]
            PgValue::BigDecimal(bigdecimal) => $query.bind(bigdecimal),
            #[cfg(feature = "decimal")]
            PgValue::Decimal(decimal) => $query.bind(decimal),
            #[cfg(feature = "chrono")]
            PgValue::UtcDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            PgValue::LocalDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            PgValue::NaiveDateTime(datetime) => $query.bind(datetime),
            #[cfg(feature = "chrono")]
            PgValue::NaiveDate(date) => $query.bind(date),
            #[cfg(feature = "chrono")]
            PgValue::NaiveTime(time) => $query.bind(time),
            #[cfg(feature = "time")]
            PgValue::PgTimeTz(timetz) => $query.bind(timetz),
            #[cfg(feature = "ipnetwork")]
            PgValue::IpNetwork(ipnetwork) => $query.bind(ipnetwork),
        }
    }
}

#[cfg(feature = "postgres")]
impl<'a> super::TryBind<'a, sqlx::Postgres>
    for sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = PgValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}

#[cfg(feature = "postgres")]
impl<'a, O> super::TryBind<'a, sqlx::Postgres>
    for sqlx::query::QueryAs<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = PgValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}
