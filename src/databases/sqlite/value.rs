use std::borrow::Cow;
use std::convert::TryFrom;

use crate::ast::Value;

/// Values that supported by SQLite.
#[cfg(feature = "sqlite")]
pub enum SQLiteValue<'a, J: serde::ser::Serialize = ()> {
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

    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// A JSON value.
    Json(Option<sqlx::types::Json<J>>),
    #[cfg(feature = "uuid")]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
    /// An UUID value.
    Uuid(Option<sqlx::types::Uuid>),

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

#[cfg(feature = "sqlite")]
impl<'a> TryFrom<Value<'a>> for SQLiteValue<'a> {
    type Error = crate::error::Error;
    fn try_from(v: Value<'a>) -> crate::Result<Self> {
        match v {
            Value::I8(int8) => Ok(SQLiteValue::I8(int8)),
            Value::I16(int16) => Ok(SQLiteValue::I16(int16)),
            Value::I32(int32) => Ok(SQLiteValue::I32(int32)),
            Value::I64(int64) => Ok(SQLiteValue::I64(int64)),
            Value::U8(uint8) => Ok(SQLiteValue::U8(uint8)),
            Value::U16(uint16) => Ok(SQLiteValue::U16(uint16)),
            Value::U32(uint32) => Ok(SQLiteValue::U32(uint32)),
            #[cfg(feature = "mysql")]
            Value::U64(_) => {
                let msg = "u64 are not supported by SQLx with SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            Value::Float(float) => Ok(SQLiteValue::Float(float)),
            Value::Double(double) => Ok(SQLiteValue::Double(double)),
            Value::Boolean(boolean) => Ok(SQLiteValue::Boolean(boolean)),
            Value::Text(text) => Ok(SQLiteValue::Text(text)),
            Value::Bytes(bytes) => Ok(SQLiteValue::Bytes(bytes)),
            #[cfg(feature = "json")]
            Value::Json(json) => Ok(SQLiteValue::Json(json)),
            #[cfg(feature = "uuid")]
            Value::Uuid(uuid) => Ok(SQLiteValue::Uuid(uuid)),
            #[cfg(feature = "postgres")]
            Value::PgInterval(_) => {
                let msg = "PgInterval are only supported by SQLx with PostgreSQL, but not SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "postgres")]
            Value::PgMoney(_) => {
                let msg = "PgMoney are only supported by SQLx with PostgreSQL, but not SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(all(feature = "time", feature = "postgres"))]
            Value::PgTimeTz(_) => {
                let msg = "PgTimeTz are only supported by SQLx with PostgreSQL, but not SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "ipnetwork")]
            Value::IpNetwork(_) => {
                let msg = "IpNetwork are only supported by SQLx with PostgreSQL, but not SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "bigdecimal")]
            Value::BigDecimal(_) => {
                let msg = "BigDecimal are not supported by SQLx with SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "decimal")]
            Value::Decimal(_decimal) => {
                let msg = "Decimal are not supported by SQLx with SQLite.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "chrono")]
            Value::UtcDateTime(dt) => Ok(SQLiteValue::UtcDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::LocalDateTime(dt) => Ok(SQLiteValue::LocalDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDateTime(dt) => Ok(SQLiteValue::NaiveDateTime(dt)),
            #[cfg(feature = "chrono")]
            Value::NaiveDate(date) => Ok(SQLiteValue::NaiveDate(date)),
            #[cfg(feature = "chrono")]
            Value::NaiveTime(time) => Ok(SQLiteValue::NaiveTime(time)),
        }
    }
}
