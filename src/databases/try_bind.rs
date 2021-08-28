pub trait TryBind<'a, DB>
where
    DB: sqlx::Database,
{
    fn try_bind(self, v: crate::ast::Value<'a>) -> crate::Result<Self>
    where
        Self: Sized;
}
