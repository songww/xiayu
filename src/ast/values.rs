
use std::borrow::{Cow};
use std::convert::TryFrom;




#[cfg(feature = "chrono")]
use sqlx::types::chrono::{self, DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
#[cfg(feature = "bigdecimal")]
use sqlx::types::BigDecimal;
#[cfg(feature = "uuid")]
use sqlx::types::Uuid;

use crate::ast::*;


/// A value written to the query as-is without parameterization.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw<'a>(pub(crate) Value<'a>);

/// Converts the value into a state to skip parameterization.
///
/// Must be used carefully to avoid SQL injections.
pub trait IntoRaw<'a> {
    fn raw(self) -> Raw<'a>;
}

impl<'a, T> IntoRaw<'a> for T
where
    T: Into<Value<'a>>,
{
    fn raw(self) -> Raw<'a> {
        Raw(self.into())
    }
}

#[cfg(feature = "postgres")]
trait PgCompatibleType:
    for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send
{
}

// #[cfg(feature = "postgres")]
// trait IntoPgCompatibleType {
//     fn into(self) -> PgCompatibleType;
// }
//
// #[cfg(feature = "postgres")]
// impl IntoPgCompatibleType for PgCompatibleType + Sized {
//     fn into(&self) -> Self {
//         self
//     }
// }

/// A value we must parameterize for the prepared statement. Null values should be
/// defined by their corresponding type variants with a `None` value for best
/// compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a, J: serde::ser::Serialize = ()> {
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
    #[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres")))
    )]
    Bytes(Option<Cow<'a, [u8]>>),
    /// TINYINT(1), BOOLEAN -> Boolean value.
    Boolean(Option<bool>),

    #[cfg(any(docsrs, feature = "mysql", feature = "sqlite"))]
    #[cfg_attr(docsrs, doc(cfg(all(any(feature = "mysql", feature = "sqlite"),))))]
    /// TINYINT UNSIGNED
    U8(Option<u8>),
    #[cfg(any(docsrs, feature = "mysql", feature = "sqlite"))]
    #[cfg_attr(docsrs, doc(cfg(all(any(feature = "mysql", feature = "sqlite"),))))]
    /// SMALLINT UNSIGNED
    U16(Option<u16>),
    #[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres")))
    )]
    /// INT UNSIGNED
    U32(Option<u32>),
    #[cfg(feature = "mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
    /// BIGINT UNSIGNED
    U64(Option<u64>),
    /*
    /// Database enum value.
    Enum(Option<Cow<'a, str>>),
    /// A single character.
    Char(Option<char>),
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    /// An array value (PostgreSQL).
    Array(Option<Vec<Value<'a>>>),
    /// A numeric value.
    /// A XML value.
    Xml(Option<Cow<'a, str>>),
    */
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// A JSON value.
    Json(Option<sqlx::types::Json<J>>),
    #[cfg(feature = "uuid")]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
    /// An UUID value.
    Uuid(Option<Uuid>),

    #[cfg(feature = "postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    /// INTERVAL
    PgInterval(Option<sqlx::postgres::types::PgInterval>),
    /*
    #[cfg(feature = "postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    /// INT8RANGE, INT4RANGE, TSRANGE, TSTZTRANGE, DATERANGE, NUMRANGE
    PgRange(Option<sqlx::postgres::types::PgRange<Box<Value<'a>>>>),
    */
    /// MONEY
    #[cfg(feature = "postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    PgMoney(Option<sqlx::postgres::types::PgMoney>),

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
    UtcDateTime(Option<DateTime<Utc>>),
    /// TIMESTAMPTZ
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    LocalDateTime(Option<DateTime<Local>>),
    /// TIMESTAMP
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveDateTime(Option<NaiveDateTime>),
    /// DATE
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveDate(Option<NaiveDate>),
    /// TIME
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    NaiveTime(Option<NaiveTime>),
    /// TIMETZ
    #[cfg(all(feature = "time", feature = "postgres"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "time", feature = "postgres"))))]
    PgTimeTz(Option<sqlx::postgres::types::PgTimeTz>),
    /// INET, CIDR
    #[cfg(feature = "ipnetwork")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ipnetwork")))]
    IpNetwork(Option<sqlx::types::ipnetwork::IpNetwork>),
}

/*
pub(crate) struct Params<'a>(pub(crate) &'a [Value<'a>]);

impl<'a> fmt::Display for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();

        write!(f, "[")?;
        for (i, val) in self.0.iter().enumerate() {
            write!(f, "{}", val)?;

            if i < (len - 1) {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            Value::Integer(val) => val.map(|v| write!(f, "{}", v)),
            Value::Float(val) => val.map(|v| write!(f, "{}", v)),
            Value::Double(val) => val.map(|v| write!(f, "{}", v)),
            Value::Text(val) => val.as_ref().map(|v| write!(f, "\"{}\"", v)),
            Value::Bytes(val) => val.as_ref().map(|v| write!(f, "<{} bytes blob>", v.len())),
            Value::Enum(val) => val.as_ref().map(|v| write!(f, "\"{}\"", v)),
            Value::Boolean(val) => val.map(|v| write!(f, "{}", v)),
            Value::Char(val) => val.map(|v| write!(f, "'{}'", v)),
            Value::Array(vals) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{}", val)?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            Value::Xml(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "json")]
            Value::Json(val) => match val {
                Json::JsonValue(jv) => jv.as_ref().map(|v| write!(f, "{:?}", v)),
                Json::JsonRawValue(jv) => jv.as_ref().map(|v| write!(f, "{:?}", v)),
            },
            #[cfg(feature = "uuid")]
            Value::Uuid(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono")]
            Value::DateTime(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono")]
            Value::Date(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono")]
            Value::Time(val) => val.map(|v| write!(f, "{}", v)),
            _ => unimplemented!(),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        let res = match pv {
            Value::Integer(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            Value::Float(f) => f.map(|f| match Number::from_f64(f as f64) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            Value::Double(f) => f.map(|f| match Number::from_f64(f) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            Value::Text(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Bytes(bytes) => {
                bytes.map(|bytes| serde_json::Value::String(base64::encode(&bytes)))
            }
            Value::Enum(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Boolean(b) => b.map(serde_json::Value::Bool),
            Value::Char(c) => c.map(|c| {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }),
            Value::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Array(v) => v.map(|v| {
                serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect())
            }),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(d) => d.map(|d| serde_json::to_value(d.to_f64().unwrap()).unwrap()),
            #[cfg(feature = "json")]
            Value::Json(v) => match v {
                Json::JsonValue(v) => v,
                Json::JsonRawValue(v) => v.and_then(|v| serde_json::to_value(*v).ok()),
            },
            #[cfg(feature = "uuid")]
            Value::Uuid(u) => u.map(|u| serde_json::Value::String(u.to_hyphenated().to_string())),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            #[cfg(feature = "chrono")]
            Value::Date(date) => date.map(|date| serde_json::Value::String(format!("{}", date))),
            #[cfg(feature = "chrono")]
            Value::Time(time) => time.map(|time| serde_json::Value::String(format!("{}", time))),
            _ => unimplemented!(),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}
*/

impl<'a> Value<'a> {
    /// Creates a new float value.
    pub const fn float(value: f32) -> Self {
        Self::Float(Some(value))
    }

    /// Creates a new double value.
    pub const fn double(value: f64) -> Self {
        Self::Double(Some(value))
    }

    /// Creates a new string value.
    pub fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::Text(Some(value.into()))
    }

    /// Creates a new bytes value.
    #[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres")))
    )]
    pub fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        Value::Bytes(Some(value.into()))
    }

    /// Creates a new boolean value.
    pub fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        Value::Boolean(Some(value.into()))
    }

    /*
    /// Creates a new array value.
    pub fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        Value::Array(Some(value.into_iter().map(|v| v.into()).collect()))
    }
    */

    /// Creates a new uuid value.
    #[cfg(feature = "uuid")]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
    pub const fn uuid(value: Uuid) -> Self {
        Value::Uuid(Some(value))
    }

    /*
    /// Creates a new datetime value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    pub const fn datetime(value: DateTime<Utc>) -> Self {
        Value::DateTime(Some(value))
    }

    /// Creates a new date value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    pub const fn date(value: NaiveDate) -> Self {
        Value::Date(Some(value))
    }

    /// Creates a new time value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    pub const fn time(value: NaiveTime) -> Self {
        Value::Time(Some(value))
    }

    /// Creates a new JSON value.
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub const fn json(value: Json<'a>) -> Self {
        Value::Json(value)
    }

    /// Creates a new XML value.
    pub fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::Xml(Some(value.into()))
    }

    /// `true` if the `Value` is null.
    pub const fn is_null(&self) -> bool {
        match self {
            Value::Integer(i) => i.is_none(),
            Value::Float(i) => i.is_none(),
            Value::Double(i) => i.is_none(),
            Value::Text(t) => t.is_none(),
            Value::Enum(e) => e.is_none(),
            Value::Bytes(b) => b.is_none(),
            Value::Boolean(b) => b.is_none(),
            Value::Char(c) => c.is_none(),
            Value::Array(v) => v.is_none(),
            Value::Xml(s) => s.is_none(),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(r) => r.is_none(),
            #[cfg(feature = "uuid")]
            Value::Uuid(u) => u.is_none(),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.is_none(),
            #[cfg(feature = "chrono")]
            Value::Date(d) => d.is_none(),
            #[cfg(feature = "chrono")]
            Value::Time(t) => t.is_none(),
            #[cfg(feature = "json")]
            Value::Json(json) => json.is_none(),
            _ => todo!(),
        }
    }

    /// `true` if the `Value` is text.
    pub const fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(Some(cow)) => Some(cow.borrow()),
            Value::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub const fn as_char(&self) -> Option<char> {
        match self {
            Value::Char(c) => *c,
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::Text(Some(cow)) => Some(cow.to_string()),
            Value::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self {
            Value::Text(Some(cow)) => Some(cow.into_owned()),
            Value::Bytes(Some(cow)) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Value::Bytes(_))
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Text(Some(cow)) => Some(cow.as_ref().as_bytes()),
            Value::Bytes(Some(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::Text(Some(cow)) => Some(cow.to_string().into_bytes()),
            Value::Bytes(Some(cow)) => Some(cow.to_owned().into()),
            _ => None,
        }
    }

    /// `true` if the `Value` is an integer.
    pub const fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }
    */

    /// Returns an `i64` if the value is an integer, otherwise `None`.
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I8(Some(i)) => Some(*i as i64),
            Value::I16(Some(i)) => Some(*i as i64),
            Value::I32(Some(i)) => Some(*i as i64),
            Value::I64(i) => *i,
            _ => None,
        }
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Double(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /*
        /// `true` if the `Value` is a numeric value or can be converted to one.
        #[cfg(feature = "bigdecimal")]
        #[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
        pub const fn is_numeric(&self) -> bool {
            matches!(self, Value::Numeric(_) | Value::Float(_) | Value::Double(_))
        }

        /// Returns a bigdecimal, if the value is a numeric, float or double value,
        /// otherwise `None`.
        #[cfg(feature = "bigdecimal")]
        #[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
        pub fn into_numeric(self) -> Option<BigDecimal> {
            match self {
                Value::Numeric(d) => d,
                Value::Float(f) => f.and_then(BigDecimal::from_f32),
                Value::Double(f) => f.and_then(BigDecimal::from_f64),
                _ => None,
            }
        }

        /// Returns a reference to a bigdecimal, if the value is a numeric.
        /// Otherwise `None`.
        #[cfg(feature = "bigdecimal")]
        #[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
        pub const fn as_numeric(&self) -> Option<&BigDecimal> {
            match self {
                Value::Numeric(d) => d.as_ref(),
                _ => None,
            }
        }

        /// `true` if the `Value` is a boolean value.
        pub const fn is_bool(&self) -> bool {
            match self {
                Value::Boolean(_) => true,
                // For schemas which don't tag booleans
                Value::Integer(Some(i)) if *i == 0 || *i == 1 => true,
                _ => false,
            }
        }

        /// Returns a bool if the value is a boolean, otherwise `None`.
        pub const fn as_bool(&self) -> Option<bool> {
            match self {
                Value::Boolean(b) => *b,
                // For schemas which don't tag booleans
                Value::Integer(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
                _ => None,
            }
        }

        /// `true` if the `Value` is an Array.
        pub const fn is_array(&self) -> bool {
            matches!(self, Value::Array(_))
        }

        /// `true` if the `Value` is of UUID type.
        #[cfg(feature = "uuid")]
        #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
        pub const fn is_uuid(&self) -> bool {
            matches!(self, Value::Uuid(_))
        }

        /// Returns an UUID if the value is of UUID type, otherwise `None`.
        #[cfg(feature = "uuid")]
        #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
        pub const fn as_uuid(&self) -> Option<Uuid> {
            match self {
                Value::Uuid(u) => *u,
                _ => None,
            }
        }

        /// `true` if the `Value` is a DateTime.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn is_datetime(&self) -> bool {
            matches!(self, Value::DateTime(_))
        }

        /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn as_datetime(&self) -> Option<DateTime<Utc>> {
            match self {
                Value::DateTime(dt) => *dt,
                _ => None,
            }
        }

        /// `true` if the `Value` is a Date.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn is_date(&self) -> bool {
            matches!(self, Value::Date(_))
        }

        /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn as_date(&self) -> Option<NaiveDate> {
            match self {
                Value::Date(dt) => *dt,
                _ => None,
            }
        }

        /// `true` if the `Value` is a `Time`.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn is_time(&self) -> bool {
            matches!(self, Value::Time(_))
        }

        /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
        #[cfg(feature = "chrono")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
        pub const fn as_time(&self) -> Option<NaiveTime> {
            match self {
                Value::Time(time) => *time,
                _ => None,
            }
        }

        /// `true` if the `Value` is a JSON value.
        #[cfg(feature = "json")]
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub const fn is_json(&self) -> bool {
            matches!(self, Value::Json(_))
        }

        /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
        #[cfg(feature = "json")]
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub const fn as_json(&self) -> Option<&Json> {
            match self {
                Value::Json(j) => j,
                _ => None,
            }
        }

        /// Transforms to a JSON Value if of Json type, otherwise `None`.
        #[cfg(feature = "json")]
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub fn into_json(self) -> Option<Json> {
            match self {
                Value::Json(j) => *j,
                _ => None,
            }
        }

        /// Returns a Vec<T> if the value is an array of T, otherwise `None`.
        pub fn into_vec<T>(self) -> Option<Vec<T>>
        where
            // Implement From<Value>
            T: TryFrom<Value<'a>>,
        {
            match self {
                Value::Array(Some(vec)) => {
                    let rslt: Result<Vec<_>, _> = vec.into_iter().map(T::try_from).collect();
                    match rslt {
                        Err(_) => None,
                        Ok(values) => Some(values),
                    }
                }
                _ => None,
            }
        }
    */
}

value!(val: i8, I8, val);
value!(val: i16, I16, val);
value!(val: i32, I32, val);
value!(val: i64, I64, val);
#[cfg(any(features = "mysql", feature = "sqlite"))]
#[cfg_attr(docsrs, doc(cfg(any(features = "mysql", feature = "sqlite"))))]
value!(val: u8, U8, val);
#[cfg(any(features = "mysql", feature = "sqlite"))]
#[cfg_attr(docsrs, doc(cfg(any(features = "mysql", feature = "sqlite"))))]
value!(val: u16, U16, val);
#[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres")))
)]
value!(val: u32, U32, val);
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
value!(val: u64, U64, val);
value!(val: bool, Boolean, val);
value!(val: &'a str, Text, val.into());
value!(val: String, Text, val.into());
value!(val: usize, I64, i64::try_from(val).unwrap());
#[cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "mysql", feature = "sqlite", feature = "postgres")))
)]
value!(val: &'a [u8], Bytes, val.into());
value!(val: f64, Double, val);
value!(val: f32, Float, val);

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
value!(val: chrono::NaiveTime, NaiveTime, val);
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
value!(val: chrono::NaiveDate, NaiveDate, val);
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
value!(val: chrono::NaiveDateTime, NaiveDateTime, val);
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
value!(val: chrono::DateTime<chrono::Utc>, UtcDateTime, val);
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
value!(val: chrono::DateTime<chrono::Local>, LocalDateTime, val);
#[cfg(feature = "bigdecimal")]
value!(val: BigDecimal, BigDecimal, val);
#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
value!(val: Uuid, Uuid, val);

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<'a, T> From<T> for Value<'a, T>
where
    T: serde::ser::Serialize,
{
    fn from(val: T) -> Self {
        Value::Json(Some(sqlx::types::Json(val)))
    }
}

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<'a, T> From<sqlx::types::Json<T>> for Value<'a, T>
where
    T: serde::ser::Serialize,
{
    fn from(val: sqlx::types::Json<T>) -> Self {
        Value::Json(Some(val))
    }
}

/*
impl<'a> TryFrom<Value<'a>> for i64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i64, Self::Error> {
        value
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i64")).build())
    }
}

#[cfg(feature = "bigdecimal")]
impl<'a> TryFrom<Value<'a>> for BigDecimal {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<BigDecimal, Self::Error> {
        value
            .into_numeric()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a decimal")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<f64, Self::Error> {
        value
            .as_f64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a f64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for String {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a string")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<bool, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a bool")).build())
    }
}

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl<'a> TryFrom<Value<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a datetime")).build())
    }
}
*/

/// An in-memory temporary table. Can be used in some of the databases in a
/// place of an actual table. Doesn't work in MySQL 5.7.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values<'a> {
    pub(crate) rows: Vec<Row<'a>>,
}

impl<'a> Values<'a> {
    /// Create a new empty in-memory set of values.
    pub fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    /// Create a new in-memory set of values.
    pub fn new(rows: Vec<Row<'a>>) -> Self {
        Self { rows }
    }

    /// Create a new in-memory set of values with an allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
        }
    }

    /// Add value to the temporary table.
    pub fn push<T>(&mut self, row: T)
    where
        T: Into<Row<'a>>,
    {
        self.rows.push(row.into());
    }

    /// The number of rows in the in-memory table.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True if has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn row_len(&self) -> usize {
        match self.rows.split_first() {
            Some((row, _)) => row.len(),
            None => 0,
        }
    }

    pub fn flatten_row(self) -> Option<Row<'a>> {
        let mut result = Row::with_capacity(self.len());

        for mut row in self.rows.into_iter() {
            match row.pop() {
                Some(value) => result.push(value),
                None => return None,
            }
        }

        Some(result)
    }
}

impl<'a, I, R> From<I> for Values<'a>
where
    I: Iterator<Item = R>,
    R: Into<Row<'a>>,
{
    fn from(rows: I) -> Self {
        Self {
            rows: rows.map(|r| r.into()).collect(),
        }
    }
}

impl<'a> IntoIterator for Values<'a> {
    type Item = Row<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "chrono")]
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1]);
        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1.0]);
        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");
        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::array(vec!["test"]);
        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");
        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![true]);
        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");
        assert_eq!(values, vec![true]);
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::array(vec![datetime]);
        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");
        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::array(vec![1]);
        let rslt: Option<Vec<f64>> = pv.into_vec();
        assert!(rslt.is_none());
    }
}
