mod aggregate_to_string;
mod average;
mod coalesce;
mod count;
#[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
mod json_extract;
mod lower;
mod maximum;
mod minimum;
mod row_number;
#[cfg(all(feature = "json", feature = "postgres"))]
mod row_to_json;
#[cfg(feature = "postgres")]
mod search;
mod sum;
mod upper;

pub use aggregate_to_string::*;
pub use average::*;
pub use coalesce::*;
pub use count::*;
#[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
pub use json_extract::*;
pub use lower::*;
pub use maximum::*;
pub use minimum::*;
pub use row_number::*;
#[cfg(all(feature = "json", feature = "postgres"))]
pub use row_to_json::*;
#[cfg(feature = "postgres")]
pub use search::*;
pub use sum::*;
pub use upper::*;

use super::{Aliasable, Expression};
use std::borrow::Cow;

/// A database function definition
#[derive(Debug, Clone, PartialEq)]
pub struct Function<'a> {
    pub(crate) typ_: FunctionType<'a>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

/// A database function type
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FunctionType<'a> {
    #[cfg(all(feature = "json", feature = "postgres"))]
    RowToJson(RowToJson<'a>),
    RowNumber(RowNumber<'a>),
    Count(Count<'a>),
    AggregateToString(AggregateToString<'a>),
    Average(Average<'a>),
    Sum(Sum<'a>),
    Lower(Lower<'a>),
    Upper(Upper<'a>),
    Minimum(Minimum<'a>),
    Maximum(Maximum<'a>),
    Coalesce(Coalesce<'a>),
    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    JsonExtract(JsonExtract<'a>),
    #[cfg(feature = "postgres")]
    TextSearch(TextSearch<'a>),
}

impl<'a> Aliasable<'a> for Function<'a> {
    type Target = Function<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

#[cfg(all(feature = "json", feature = "postgres"))]
function!(RowToJson);

#[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
function!(JsonExtract);

#[cfg(feature = "postgres")]
function!(TextSearch);

function!(
    RowNumber,
    Count,
    AggregateToString,
    Average,
    Sum,
    Lower,
    Upper,
    Minimum,
    Maximum,
    Coalesce
);
