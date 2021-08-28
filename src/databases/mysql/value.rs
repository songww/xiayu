use std::borrow::Cow;
use std::convert::TryFrom;

use crate::ast::Value;

/// Values that supported by MySQL.
#[cfg(feature = "mysql")]
pub enum MyValue<'a, J: serde::ser::Serialize = ()> {
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

    /// TINYINT UNSIGNED
    U8(Option<u8>),
    /// SMALLINT UNSIGNED
    U16(Option<u16>),
    /// INT UNSIGNED
    U32(Option<u32>),
    /// BIGINT UNSIGNED
    U64(Option<u64>),

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
}

#[cfg(feature = "mysql")]
impl<'a> TryFrom<Value<'a>> for MyValue<'a> {
    type Error = crate::error::Error;
    fn try_from(v: Value<'a>) -> crate::Result<Self> {
        match v {
            Value::I8(int8) => Ok(MyValue::I8(int8)),
            Value::I16(int16) => Ok(MyValue::I16(int16)),
            Value::I32(int32) => Ok(MyValue::I32(int32)),
            Value::I64(int64) => Ok(MyValue::I64(int64)),
            Value::U8(uint8) => Ok(MyValue::U8(uint8)),
            Value::U16(uint16) => Ok(MyValue::U16(uint16)),
            Value::U32(uint32) => Ok(MyValue::U32(uint32)),
            Value::U64(uint64) => Ok(MyValue::U64(uint64)),
            Value::Float(float) => Ok(MyValue::Float(float)),
            Value::Double(double) => Ok(MyValue::Double(double)),
            Value::Boolean(boolean) => Ok(MyValue::Boolean(boolean)),
            Value::Text(text) => Ok(MyValue::Text(text)),
            Value::Bytes(bytes) => Ok(MyValue::Bytes(bytes)),
            #[cfg(feature = "json")]
            Value::Json(json) => Ok(MyValue::Json(json)),
            #[cfg(feature = "uuid")]
            Value::Uuid(uuid) => Ok(MyValue::Uuid(uuid)),
            #[cfg(feature = "postgres")]
            Value::PgInterval(_) => {
                let msg = "PgInterval are only supported by SQLx with PostgreSQL, but not MySQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "postgres")]
            Value::PgMoney(_) => {
                let msg = "PgMoney are only supported by SQLx with PostgreSQL, but not MySQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(all(feature = "time", feature = "postgres"))]
            Value::PgTimeTz(_) => {
                let msg = "PgTimeTz are only supported by SQLx with PostgreSQL, but not MySQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "ipnetwork")]
            Value::IpNetwork(_) => {
                let msg = "IpNetwork are only supported by SQLx with PostgreSQL, but not MySQL.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "bigdecimal")]
            Value::BigDecimal(bigdecimal) => Ok(MyValue::BigDecimal(bigdecimal)),
            #[cfg(feature = "decimal")]
            Value::Decimal(decimal) => Ok(MyValue::Decimal(decimal)),
            #[cfg(feature = "chrono")]
            Value::UtcDateTime(dt) => Ok(MyValue::UtcDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::LocalDateTime(dt) => Ok(MyValue::LocalDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDateTime(dt) => Ok(MyValue::NaiveDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDate(date) => Ok(MyValue::NaiveDate(date)),
            #[cfg(feature = "chrono")]
            Value::NaiveTime(time) => Ok(MyValue::NaiveTime(time)),
        }
    }
}
