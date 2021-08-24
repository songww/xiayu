use crate::prelude::Column;

#[derive(Debug, Clone, PartialEq)]
/// Holds the columns on which to perform a full-text search
pub struct TextSearch<'a> {
    pub(crate) columns: Vec<Column<'a>>,
}

/// Performs a full-text search. Use it in combination with the `.matches()` comparable.
///
/// ```rust
/// # use xiayu::{prelude::*, visitors::{Visitor, Postgres}};
/// # fn main() -> Result<(), xiayu::error::Error> {
/// #[derive(Entity)]
/// struct Recipe {
///   name: String,
///   ingredients: String,
/// }
/// let search: Expression = text_search(&[Recipe::name, Recipe::ingredients]).into();
/// let query = Select::from_table(Recipe::table()).so_that(search.matches("chicken"));
/// let (sql, params) = Postgres::build(query)?;
///
/// assert_eq!(
///    "SELECT \"recipes\".* FROM \"recipes\" \
///     WHERE to_tsvector(\"name\"|| ' ' ||\"ingredients\") @@ to_tsquery($1)", sql
/// );
///
/// assert_eq!(params, vec![Value::from("chicken")]);
/// # Ok(())    
/// # }
/// ```
#[cfg(feature = "postgres")]
pub fn text_search<'a, T: Clone>(columns: &[T]) -> super::Function<'a>
where
    T: Into<Column<'a>>,
{
    let columns: Vec<Column> = columns.iter().map(|c| c.clone().into()).collect();
    let fun = TextSearch { columns };

    fun.into()
}
