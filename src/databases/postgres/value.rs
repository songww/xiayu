use std::borrow::Cow;
use std::convert::TryFrom;

use crate::ast::Value;

/// Values that supported by PostgreSQL.
#[cfg(feature = "postgres")]
pub enum PgValue<'a, J: serde::ser::Serialize = ()> {
    /// TINYINT
    I8(Option<i8>),
    /// SMALLINT
    I16(Option<i16>),
    /// INT
    I32(Option<i32>),
    /// BIGINT
    I64(Option<i64>),
    /// FLOAT -> 32-bit floating point.
    Float(Option<f32>),
    /// DOUBLE -> 64-bit floating point.
    Double(Option<f64>),
    /// VARCHAR, CHAR, TEXT -> String value.
    Text(Option<Cow<'a, str>>),
    /// VARBINARY, BINARY, BLOB -> Bytes value.
    Bytes(Option<Cow<'a, [u8]>>),
    /// TINYINT(1), BOOLEAN -> Boolean value.
    Boolean(Option<bool>),

    /// INT UNSIGNED
    U32(Option<u32>),

    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// A JSON value.
    Json(Option<sqlx::types::Json<J>>),
    #[cfg(feature = "uuid")]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
    /// An UUID value.
    Uuid(Option<sqlx::types::Uuid>),

    /// A numeric value.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
    BigDecimal(Option<sqlx::types::BigDecimal>),
    /// A numeric value.
    #[cfg(feature = "decimal")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decimal")))]
    Decimal(Option<sqlx::types::Decimal>),

    /// TIMESTAMPTZ
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    UtcDateTime(Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>),
    /// TIMESTAMPTZ
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    LocalDateTime(Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Local>>),
    /// TIMESTAMP
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveDateTime(Option<sqlx::types::chrono::NaiveDateTime>),
    /// DATE
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveDate(Option<sqlx::types::chrono::NaiveDate>),
    /// TIME
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveTime(Option<sqlx::types::chrono::NaiveTime>),

    /// INTERVAL
    PgInterval(Option<sqlx::postgres::types::PgInterval>),
    /// MONEY
    PgMoney(Option<sqlx::postgres::types::PgMoney>),
    /// TIMETZ
    #[cfg(feature = "time")]
    PgTimeTz(Option<sqlx::postgres::types::PgTimeTz>),
    /// INET, CIDR
    #[cfg(feature = "ipnetwork")]
    IpNetwork(Option<sqlx::types::ipnetwork::IpNetwork>),
}

#[cfg(feature = "postgres")]
impl<'a> TryFrom<Value<'a>> for PgValue<'a> {
    type Error = crate::error::Error;
    fn try_from(v: Value<'a>) -> crate::Result<Self> {
        match v {
            Value::I8(int8) => Ok(PgValue::I8(int8)),
            Value::I16(int16) => Ok(PgValue::I16(int16)),
            Value::I32(int32) => Ok(PgValue::I32(int32)),
            Value::I64(int64) => Ok(PgValue::I64(int64)),
            #[cfg(any(feature = "mysql", feature = "sqlite"))]
            Value::U8(_uint8) => {
                let msg = "u8 are not supported by SQLx with PostgreSQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(any(feature = "mysql", feature = "sqlite"))]
            Value::U16(_uint16) => {
                let msg = "u16 are not supported by SQLx with PostgreSQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            Value::U32(uint32) => Ok(PgValue::U32(uint32)),
            #[cfg(feature = "mysql")]
            Value::U64(_) => {
                let msg = "u64 are not supported by SQLx with PostgreSQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            Value::Float(float) => Ok(PgValue::Float(float)),
            Value::Double(double) => Ok(PgValue::Double(double)),
            Value::Boolean(boolean) => Ok(PgValue::Boolean(boolean)),
            Value::Text(text) => Ok(PgValue::Text(text)),
            Value::Bytes(bytes) => Ok(PgValue::Bytes(bytes)),
            #[cfg(feature = "json")]
            Value::Json(json) => Ok(PgValue::Json(json)),
            #[cfg(feature = "uuid")]
            Value::Uuid(uuid) => Ok(PgValue::Uuid(uuid)),
            Value::PgInterval(interval) => Ok(PgValue::PgInterval(interval)),
            Value::PgMoney(money) => Ok(PgValue::PgMoney(money)),
            #[cfg(feature = "time")]
            Value::PgTimeTz(timetz) => Ok(PgValue::PgTimeTz(timetz)),
            #[cfg(feature = "ipnetwork")]
            Value::IpNetwork(ipnetwork) => Ok(PgValue::IpNetwork(ipnetwork)),
            #[cfg(feature = "bigdecimal")]
            Value::BigDecimal(bigdecimal) => Ok(PgValue::BigDecimal(bigdecimal)),
            #[cfg(feature = "decimal")]
            Value::Decimal(decimal) => Ok(PgValue::Decimal(decimal)),
            #[cfg(feature = "chrono")]
            Value::UtcDateTime(dt) => Ok(PgValue::UtcDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::LocalDateTime(dt) => Ok(PgValue::LocalDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDateTime(dt) => Ok(PgValue::NaiveDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDate(date) => Ok(PgValue::NaiveDate(date)),
            #[cfg(feature = "chrono")]
            Value::NaiveTime(time) => Ok(PgValue::NaiveTime(time)),
        }
    }
}
