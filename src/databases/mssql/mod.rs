use std::convert::TryFrom;

mod value;

pub use value::MsValue;

#[cfg(feature = "mssql")]
impl<'a> super::HasVisitor<'a> for sqlx::Mssql {
    type Visitor = crate::visitors::Mssql<'a>;
    fn visitor() -> Self::Visitor {
        Self::Visitor::default()
    }
}

macro_rules! bind {
    ($query:ident, $value: ident) => {
        match $value {
            MsValue::I8(int8) => $query.bind(int8),
            MsValue::I16(int16) => $query.bind(int16),
            MsValue::I32(int32) => $query.bind(int32),
            MsValue::I64(int64) => $query.bind(int64),
            MsValue::Float(float) => $query.bind(float) ,
            MsValue::Double(double) => $query.bind(double),
            MsValue::Boolean(boolean) => $query.bind(boolean),
            MsValue::Text(text) => $query.bind(text.map(|text|text.into_owned()))
        }
    }
}

#[cfg(feature = "mssql")]
impl<'a> super::TryBind<'a, sqlx::Mssql>
    for sqlx::query::Query<'a, sqlx::Mssql, sqlx::mssql::MssqlArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = MsValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}

#[cfg(feature = "mssql")]
impl<'a, O> super::TryBind<'a, sqlx::Mssql>
    for sqlx::query::QueryAs<'a, sqlx::Mssql, O, sqlx::mssql::MssqlArguments>
{
    fn try_bind(self, value: crate::ast::Value<'a>) -> crate::Result<Self> {
        let value = MsValue::try_from(value)?;
        Ok(bind!(self, value))
    }
}
