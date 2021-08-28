use std::borrow::Cow;
use std::convert::TryFrom;

use crate::ast::Value;

/// Values that supported by SQL Server.
#[cfg(feature = "mssql")]
pub enum MsValue<'a> {
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
    /// TINYINT(1), BOOLEAN -> Boolean value.
    Boolean(Option<bool>),
}

#[cfg(feature = "mssql")]
impl<'a> TryFrom<Value<'a>> for MsValue<'a> {
    type Error = crate::error::Error;
    fn try_from(v: Value<'a>) -> crate::Result<Self> {
        match v {
            Value::I8(int8) => Ok(MsValue::I8(int8)),
            Value::I16(int16) => Ok(MsValue::I16(int16)),
            Value::I32(int32) => Ok(MsValue::I32(int32)),
            Value::I64(int64) => Ok(MsValue::I64(int64)),
            #[cfg(any(feature = "mysql", feature = "sqlite"))]
            Value::U8(_) => {
                let msg = "u8 are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(any(feature = "mysql", feature = "sqlite"))]
            Value::U16(_) => {
                let msg = "u16 are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
            Value::U32(_) => {
                let msg = "u32 are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "mysql")]
            Value::U64(_) => {
                let msg = "u64 are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            Value::Float(float) => Ok(MsValue::Float(float)),
            Value::Double(double) => Ok(MsValue::Double(double)),
            Value::Boolean(boolean) => Ok(MsValue::Boolean(boolean)),
            Value::Text(text) => Ok(MsValue::Text(text)),
            #[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
            Value::Bytes(_) => {
                let msg = "&[u8]/Vec<u8> are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            },
            #[cfg(feature = "json")]
            Value::Json(_) => {
                let msg = "Json<T> are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "uuid")]
            Value::Uuid(_) => {
                let msg = "Uuid are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "postgres")]
            Value::PgInterval(_) => {
                let msg = "PgInterval are only supported by SQLx with PostgreSQL, but not SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "postgres")]
            Value::PgMoney(_) => {
                let msg = "PgMoney are only supported by SQLx with PostgreSQL, but not SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(all(feature = "time", feature = "postgres"))]
            Value::PgTimeTz(_) => {
                let msg = "PgTimeTz are only supported by SQLx with PostgreSQL, but not SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "ipnetwork")]
            Value::IpNetwork(_) => {
                let msg = "IpNetwork are only supported by SQLx with PostgreSQL, but not SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "bigdecimal")]
            Value::BigDecimal(_) => {
                let msg = "BigDecimal are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "decimal")]
            Value::Decimal(_decimal) => {
                let msg = "Decimal are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "chrono")]
            Value::UtcDateTime(_dt) => {
                let msg = "DataTime<Utc> are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            },
            #[cfg(feature = "chrono")]
            Value::LocalDateTime(_dt) => {
                let msg = "DataTime<Local> are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "chrono")]
            Value::NaiveDateTime(_dt) => {
                let msg = "NaiveDateTime are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "chrono")]
            Value::NaiveDate(_date) => {
                let msg = "NaiveDate are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
            #[cfg(feature = "chrono")]
            Value::NaiveTime(_time) => {
                let msg = "NaiveTime are not supported by SQLx with SQL Server.";
                let kind = crate::error::ErrorKind::conversion(msg);

                let mut builder = crate::error::Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())
            }
        }
    }
}
