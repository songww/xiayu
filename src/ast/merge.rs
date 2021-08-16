use super::*;
use crate::error::*;
use std::convert::TryFrom;

/// A builder for SQL `MERGE` queries.
///
/// Not complete and not meant for external use in this state. Made for
/// compatibility purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) using: Using<'a>,
    pub(crate) when_not_matched: Option<Query<'a>>,
    pub(crate) returning: Option<Vec<Column<'a>>>,
}

impl<'a> Merge<'a> {
    pub(crate) fn new<T, U>(table: T, using: U) -> Self
    where
        T: Into<Table<'a>>,
        U: Into<Using<'a>>,
    {
        Self {
            table: table.into(),
            using: using.into(),
            when_not_matched: None,
            returning: None,
        }
    }

    pub(crate) fn when_not_matched<Q>(mut self, query: Q) -> Self
    where
        Q: Into<Query<'a>>,
    {
        self.when_not_matched = Some(query.into());
        self
    }

    pub(crate) fn returning<K, I>(mut self, columns: I) -> Self
    where
        K: Into<Column<'a>>,
        I: IntoIterator<Item = K>,
    {
        self.returning = Some(columns.into_iter().map(|k| k.into()).collect());
        self
    }
}

impl<'a> From<Merge<'a>> for Query<'a> {
    fn from(merge: Merge<'a>) -> Self {
        Self::Merge(Box::new(merge))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Using<'a> {
    pub(crate) base_query: Query<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) as_table: Table<'a>,
    pub(crate) on_conditions: ConditionTree<'a>,
}

impl<'a> Using<'a> {
    pub(crate) fn on<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.on_conditions = conditions.into();
        self
    }
}

pub(crate) trait IntoUsing<'a, E>
where
    E: crate::prelude::Entity,
{
    fn into_using(self, alias: E, columns: Vec<Column<'a>>) -> Using<'a>;
}

impl<'a, I, E> IntoUsing<'a, E> for I
where
    I: Into<Query<'a>>,
    E: crate::prelude::Entity,
{
    fn into_using(self, alias: E, columns: Vec<Column<'a>>) -> Using<'a> {
        Using {
            base_query: self.into(),
            as_table: Table::from(alias),
            columns,
            on_conditions: ConditionTree::NoCondition,
        }
    }
}

impl<'a> TryFrom<Insert<'a>> for Merge<'a> {
    type Error = Error;

    fn try_from(insert: Insert<'a>) -> crate::Result<Self> {
        let table = insert.table.ok_or_else(|| {
            let kind =
                ErrorKind::conversion("Insert needs to point to a table for conversion to Merge.");
            Error::builder(kind).build()
        })?;

        if table.index_definitions.is_empty() {
            let kind = ErrorKind::conversion(
                "Insert table needs schema metadata for conversion to Merge.",
            );
            return Err(Error::builder(kind).build());
        }

        let columns = insert.columns;

        let query = match insert.values.kind {
            ExpressionKind::Row(row) => {
                let cols_vals = columns.iter().zip(row.values.into_iter());

                let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                    query.value(val.alias(col.name.clone()))
                });

                Query::from(select)
            }
            ExpressionKind::Values(values) => {
                let mut rows = values.rows;
                let row = rows.pop().unwrap();
                let cols_vals = columns.iter().zip(row.values.into_iter());

                let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                    query.value(val.alias(col.name.clone()))
                });

                let union = rows.into_iter().fold(Union::new(select), |union, row| {
                    let cols_vals = columns.iter().zip(row.values.into_iter());

                    let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                        query.value(val.alias(col.name.clone()))
                    });

                    union.all(select)
                });

                Query::from(union)
            }
            ExpressionKind::Selection(selection) => Query::from(selection),
            _ => {
                let kind = ErrorKind::conversion("Insert type not supported.");
                return Err(Error::builder(kind).build());
            }
        };

        let bare_columns: Vec<_> = columns.clone().into_iter().map(|c| c.into_bare()).collect();

        let using = query
            .into_using(Dual::default(), bare_columns.clone())
            .on(table.join_conditions(&columns).unwrap());

        let dual_columns: Vec<_> = columns
            .into_iter()
            .map(|c| {
                let mut c = c.clone();
                c.table.replace(Table {
                    typ: TableType::Table("Dual".into()),
                    alias: None,
                    database: None,
                    index_definitions: Vec::new(),
                });
                c
            })
            .collect();
        let not_matched = Insert::multi(bare_columns).values(dual_columns);
        let mut merge = Merge::new(table, using).when_not_matched(not_matched);

        if let Some(columns) = insert.returning {
            merge = merge.returning(columns);
        }

        Ok(merge)
    }
}

#[derive(Clone, Debug, Default)]
struct Dual {
    id: i32,
}

impl crate::prelude::Entity for Dual {
    const COLUMNS: &'static [Column<'static>] = &[];

    fn tablename() -> &'static str {
        "dual"
    }

    fn columns() -> &'static [Column<'static>] {
        Self::COLUMNS
    }

    fn table() -> Table<'static> {
        Table {
            typ: TableType::Table(Self::tablename().into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        }
    }
}

impl crate::prelude::HasPrimaryKey for Dual {
    type PrimaryKey = crate::prelude::ColumnOptions<i32>;
    fn primary_key() -> Self::PrimaryKey {
        crate::prelude::ColumnOptions::new(
            "id", "dual", true, true, None, None, false, None, false, None,
        )
    }
}
