use super::Visitor;
#[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
use crate::prelude::{JsonExtract, JsonType, TableType};
use crate::{
    ast::{
        Column, Comparable, Expression, ExpressionKind, Insert, IntoRaw, Join, JoinData, Joinable,
        Merge, OnConflict, Order, Ordering, Row, Table, TypeDataLength, TypeFamily, Value, Values,
    },
    prelude::{Aliasable, Average, Query},
    visitors,
};
use std::{convert::TryFrom, fmt::Write, iter};

static GENERATED_KEYS: &str = "@generated_keys";

/// A visitor to generate queries for the SQL Server database.
///
/// The returned parameter values can be used directly with the tiberius crate.
#[cfg_attr(docsrs, doc(cfg(feature = "mssql")))]
pub struct Mssql<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
    order_by_set: bool,
}

impl<'a> Mssql<'a> {
    // TODO: figure out that merge shit
    fn visit_returning(&mut self, columns: Vec<Column<'a>>) -> visitors::Result {
        let inserted_table = Table {
            typ: crate::ast::TableType::Table("Inserted".into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        };
        let cols: Vec<_> = columns
            .into_iter()
            .map(|c| c.table(inserted_table.clone()))
            .collect();

        self.write(" OUTPUT ")?;

        let len = cols.len();
        for (i, value) in cols.into_iter().enumerate() {
            self.visit_column(value)?;

            if i < (len - 1) {
                self.write(",")?;
            }
        }

        self.write(" INTO ")?;
        self.write(GENERATED_KEYS)?;

        Ok(())
    }

    fn visit_type_family(&mut self, type_family: TypeFamily) -> visitors::Result {
        match type_family {
            TypeFamily::Text(len) => {
                self.write("NVARCHAR(")?;
                match len {
                    Some(TypeDataLength::Constant(len)) => self.write(len)?,
                    Some(TypeDataLength::Maximum) => self.write("MAX")?,
                    None => self.write(4000)?,
                }
                self.write(")")
            }
            TypeFamily::Int => self.write("BIGINT"),
            TypeFamily::Float => self.write("FLOAT(24)"),
            TypeFamily::Double => self.write("FLOAT(53)"),
            TypeFamily::Decimal(size) => {
                self.write("DECIMAL(")?;
                match size {
                    Some((p, s)) => {
                        self.write(p)?;
                        self.write(",")?;
                        self.write(s)?;
                    }
                    None => self.write("32,16")?,
                }
                self.write(")")
            }
            TypeFamily::Boolean => self.write("BIT"),
            TypeFamily::Uuid => self.write("UNIQUEIDENTIFIER"),
            TypeFamily::DateTime => self.write("DATETIMEOFFSET"),
            TypeFamily::Bytes(len) => {
                self.write("VARBINARY(")?;
                match len {
                    Some(TypeDataLength::Constant(len)) => self.write(len)?,
                    Some(TypeDataLength::Maximum) => self.write("MAX")?,
                    None => self.write(8000)?,
                }
                self.write(")")
            }
        }
    }

    fn create_generated_keys(&mut self, columns: Vec<Column<'a>>) -> visitors::Result {
        self.write("DECLARE ")?;
        self.write(GENERATED_KEYS)?;
        self.write(" table")?;

        self.surround_with("(", ")", move |this| {
            let columns_len = columns.len();

            for (i, column) in columns.into_iter().enumerate() {
                this.visit_column(column.clone())?;
                this.write(" ")?;

                match column.type_family {
                    Some(type_family) => this.visit_type_family(type_family)?,
                    None => this.write("NVARCHAR(255)")?,
                }

                if i < (columns_len - 1) {
                    this.write(",")?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn select_generated_keys(
        &mut self,
        columns: Vec<Column<'a>>,
        target_table: Table<'a>,
    ) -> visitors::Result {
        let col_len = columns.len();

        let t_table = Table {
            typ: TableType::Table("t".into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        };

        let g_table = Table {
            typ: TableType::Table("g".into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        };

        let join = columns
            .iter()
            .fold(JoinData::from(target_table.alias("t")), |acc, col| {
                let left = Column::new(col.name.to_string()).table(t_table.clone());
                let right = Column::new(col.name.to_string()).table(g_table.clone());

                acc.on((left).equals(right))
            });

        self.write("SELECT ")?;

        for (i, col) in columns.into_iter().enumerate() {
            self.visit_column(col.table(t_table.clone()))?;

            if i < (col_len - 1) {
                self.write(",")?;
            }
        }

        self.write(" FROM ")?;
        self.write(GENERATED_KEYS)?;
        self.write(" AS g")?;
        self.visit_joins(vec![Join::Inner(join)])?;

        self.write(" WHERE @@ROWCOUNT > 0")?;

        Ok(())
    }
}

impl<'a> Default for Mssql<'a> {
    fn default() -> Self {
        Mssql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
            order_by_set: false,
        }
    }
}

impl<'a> Visitor<'a> for Mssql<'a> {
    const C_BACKTICK_OPEN: &'static str = "[";
    const C_BACKTICK_CLOSE: &'static str = "]";
    const C_WILDCARD: &'static str = "%";

    #[tracing::instrument(name = "render_sql", skip(query))]
    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<crate::ast::Query<'a>>,
    {
        let mut this = Mssql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
            order_by_set: false,
        };

        Mssql::visit_query(&mut this, query.into())?;

        Ok((this.query, this.parameters))
    }

    fn write<D: std::fmt::Display>(&mut self, s: D) -> visitors::Result {
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value)
    }

    /// A point to modify an incoming query to make it compatible with the
    /// SQL Server.
    fn compatibility_modifications(&self, query: Query<'a>) -> Query<'a> {
        match query {
            // Finding possible `(a, b) (NOT) IN (SELECT x, y ...)` comparisons,
            // and replacing them with common table expressions.
            Query::Select(select) => select
                .convert_tuple_selects_to_ctes(true, &mut 0)
                .expect_left("Top-level query was right")
                .into(),
            // Replacing the `ON CONFLICT DO NOTHING` clause with a `MERGE` statement.
            Query::Insert(insert) => match insert.on_conflict {
                Some(OnConflict::DoNothing) => Merge::try_from(*insert).unwrap().into(),
                _ => Query::Insert(insert),
            },
            _ => query,
        }
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitors::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), false)?;
            }
            (left_kind, right_kind) => {
                let (l_alias, r_alias) = (left.alias, right.alias);

                let mut left = Expression::from(left_kind);

                if let Some(alias) = l_alias {
                    left = left.alias(alias);
                }

                let mut right = Expression::from(right_kind);

                if let Some(alias) = r_alias {
                    right = right.alias(alias);
                }

                self.visit_expression(left)?;

                self.write(" = ")?;

                self.visit_expression(right)?;
            }
        }

        Ok(())
    }

    fn visit_not_equals(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
    ) -> visitors::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), true)?;
            }
            (left_kind, right_kind) => {
                let (l_alias, r_alias) = (left.alias, right.alias);

                let mut left = Expression::from(left_kind);

                if let Some(alias) = l_alias {
                    left = left.alias(alias);
                }

                let mut right = Expression::from(right_kind);

                if let Some(alias) = r_alias {
                    right = right.alias(alias);
                }

                self.visit_expression(left)?;

                self.write(" <> ")?;

                self.visit_expression(right)?;
            }
        }

        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitors::Result {
        let res = match value {
            Value::I8(i) => i.map(|i| self.write(i)),
            Value::I16(i) => i.map(|i| self.write(i)),
            Value::I32(i) => i.map(|i| self.write(i)),
            Value::I64(i) => i.map(|i| self.write(i)),
            Value::Float(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f32::INFINITY => self.write("'Infinity'"),
                f if f == f32::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{:?}", v)),
            }),
            Value::Double(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f64::INFINITY => self.write("'Infinity'"),
                f if f == f64::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{:?}", v)),
            }),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Boolean(b) => b.map(|b| self.write(if b { 1 } else { 0 })),
            v @ _ => {
                // FIXME: Maybe define MsValue at here?
                crate::databases::mssql::MsValue::try_from(v)?;
                None
            }
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<Value<'a>>,
        offset: Option<Value<'a>>,
    ) -> visitors::Result {
        let add_ordering = |this: &mut Self| {
            if !this.order_by_set {
                this.write(" ORDER BY ")?;
                this.visit_ordering(Ordering::new(vec![(1.raw().into(), None)]))?;
            }

            Ok::<(), crate::error::Error>(())
        };

        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, Some(offset))
                if self.order_by_set || offset.as_i64().map(|i| i > 0).unwrap_or(false) =>
            {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS")
            }
            (Some(limit), None) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(Value::from(0))?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, _) => Ok(()),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitors::Result {
        if let Some(returning) = insert.returning.as_ref().cloned() {
            self.create_generated_keys(returning)?;
            self.write(" ")?;
        }

        self.write("INSERT")?;

        if let Some(ref table) = insert.table {
            self.write(" INTO ")?;
            self.visit_table(table.clone(), true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    if let Some(ref returning) = insert.returning {
                        self.visit_returning(returning.clone())?;
                    }

                    self.write(" DEFAULT VALUES")?;
                } else {
                    self.write(" ")?;
                    self.visit_row(Row::from(insert.columns))?;

                    if let Some(ref returning) = insert.returning {
                        self.visit_returning(returning.clone())?;
                    }

                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                self.write(" ")?;
                self.visit_row(Row::from(insert.columns))?;

                if let Some(ref returning) = insert.returning {
                    self.visit_returning(returning.clone())?;
                }

                self.write(" VALUES ")?;

                let values_len = values.len();
                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(",")?;
                    }
                }
            }
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        if let Some(returning) = insert.returning {
            let table = insert.table.unwrap();
            self.write(" ")?;
            self.select_generated_keys(returning, table)?;
        }

        Ok(())
    }

    fn visit_merge(&mut self, merge: Merge<'a>) -> visitors::Result {
        if let Some(returning) = merge.returning.as_ref().cloned() {
            self.create_generated_keys(returning)?;
            self.write(" ")?;
        }

        self.write("MERGE INTO ")?;
        self.visit_table(merge.table.clone(), true)?;

        self.write(" USING ")?;

        let base_query = merge.using.base_query;
        self.surround_with("(", ")", |ref mut s| s.visit_query(base_query))?;

        self.write(" AS ")?;
        self.visit_table(merge.using.as_table, false)?;

        self.write(" ")?;
        self.visit_row(Row::from(merge.using.columns))?;
        self.write(" ON ")?;
        self.visit_conditions(merge.using.on_conditions)?;

        if let Some(query) = merge.when_not_matched {
            self.write(" WHEN NOT MATCHED THEN ")?;
            self.visit_query(query)?;
        }

        if let Some(columns) = merge.returning {
            self.visit_returning(columns.clone())?;
            self.write("; ")?;
            self.select_generated_keys(columns, merge.table)?;
        } else {
            self.write(";")?;
        }

        Ok(())
    }

    fn parameter_substitution(&mut self) -> visitors::Result {
        self.write("@P")?;
        self.write(self.parameters.len())
    }

    fn visit_aggregate_to_string(&mut self, value: crate::ast::Expression<'a>) -> visitors::Result {
        self.write("STRING_AGG")?;
        self.surround_with("(", ")", |ref mut se| {
            se.visit_expression(value)?;
            se.write(",")?;
            se.write("\",\"")
        })
    }

    // MSSQL doesn't support tuples, we do AND/OR.
    fn visit_multiple_tuple_comparison(
        &mut self,
        left: Row<'a>,
        right: Values<'a>,
        negate: bool,
    ) -> visitors::Result {
        let row_len = left.len();
        let values_len = right.len();

        if negate {
            self.write("NOT ")?;
        }

        self.surround_with("(", ")", |this| {
            for (i, row) in right.into_iter().enumerate() {
                this.surround_with("(", ")", |se| {
                    let row_and_vals = left.values.clone().into_iter().zip(row.values.into_iter());

                    for (j, (expr, val)) in row_and_vals.enumerate() {
                        se.visit_expression(expr)?;
                        se.write(" = ")?;
                        se.visit_expression(val)?;

                        if j < row_len - 1 {
                            se.write(" AND ")?;
                        }
                    }

                    Ok(())
                })?;

                if i < values_len - 1 {
                    this.write(" OR ")?;
                }
            }

            Ok(())
        })
    }

    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> visitors::Result {
        let len = ordering.0.len();

        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            let direction = ordering.map(|dir| match dir {
                Order::Asc => " ASC",
                Order::Desc => " DESC",
            });

            self.visit_expression(value)?;
            self.write(direction.unwrap_or(""))?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        self.order_by_set = true;

        Ok(())
    }

    fn visit_average(&mut self, avg: Average<'a>) -> visitors::Result {
        self.write("AVG")?;

        // SQL Server will average as an integer, so average of 0 an 1 would be
        // 0, if we don't convert the value to a decimal first.
        self.surround_with("(", ")", |ref mut s| {
            s.write("CONVERT")?;

            s.surround_with("(", ")", |ref mut s| {
                s.write("DECIMAL(32,16),")?;
                s.visit_column(avg.column)
            })
        })?;

        Ok(())
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_extract(&mut self, _json_extract: JsonExtract<'a>) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_contains(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_begins_with(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_ends_into(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_type_equals(
        &mut self,
        _left: Expression<'a>,
        _json_type: JsonType,
    ) -> visitors::Result {
        unimplemented!("JSON_TYPE is not yet supported on MSSQL")
    }

    #[cfg(feature = "postgres")]
    fn visit_text_search(
        &mut self,
        _text_search: crate::prelude::TextSearch<'a>,
    ) -> visitors::Result {
        unimplemented!("Full-text search is not yet supported on MSSQL")
    }

    #[cfg(feature = "postgres")]
    fn visit_matches(
        &mut self,
        _left: Expression<'a>,
        _right: std::borrow::Cow<'a, str>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("Full-text search is not yet supported on MSSQL")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::*,
        val,
        visitors::{Mssql, Visitor},
    };
    use indoc::indoc;

    fn expected_values<'a, T>(sql: &'static str, params: Vec<T>) -> (String, Vec<Value<'a>>)
    where
        T: Into<Value<'a>>,
    {
        (
            String::from(sql),
            params.into_iter().map(|p| p.into()).collect(),
        )
    }

    fn default_params<'a>(mut additional: Vec<Value<'a>>) -> Vec<Value<'a>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_select_1() {
        let expected = expected_values("SELECT @P1", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT @P1 AS [test]", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT @P1 AS [test]";
        let query = Select::default().value(val!(Value::Integer(None)).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::Integer(None)], params);
    }

    #[derive(Entity)]
    #[tablename = "musti"]
    struct Musti {
        foo: i32,
        baz: i32,
        bar: i32,
        paw: String,
        nose: String,
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT [musti].* FROM [musti]";
        let query = Select::from_table(Musti::table());
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[derive(Entity)]
    #[tablename = "test"]
    struct TestEntity {
        id1: i32,
        id2: i32,
        bar: String,
    }

    #[test]
    fn test_in_values() {
        use crate::values;

        let expected_sql =
            "SELECT [test].* FROM [test] WHERE (([test].[id1] = @P1 AND [test].[id2] = @P2) OR ([test].[id1] = @P3 AND [test].[id2] = @P4))";

        let query = Select::from_table(TestEntity::table()).so_that(
            Row::from((TestEntity::id1, TestEntity::id2)).in_selection(values!((1, 2), (3, 4))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_not_in_values() {
        use crate::values;

        let expected_sql =
            "SELECT [test].* FROM [test] WHERE NOT (([test].[id1] = @P1 AND [test].[id2] = @P2) OR ([test].[id1] = @P3 AND [test].[id2] = @P4))";

        let query = Select::from_table(TestEntity::table()).so_that(
            Row::from((TestEntity::id1, TestEntity::id2)).not_in_selection(values!((1, 2), (3, 4))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_in_values_singular() {
        let mut cols = Row::new();
        cols.push(TestEntity::id1);

        let mut vals = Values::new(vec![]);

        {
            let mut row1 = Row::new();
            row1.push(1);

            let mut row2 = Row::new();
            row2.push(2);

            vals.push(row1);
            vals.push(row2);
        }

        let query = Select::from_table(TestEntity::table()).so_that(cols.in_selection(vals));
        let (sql, params) = Mssql::build(query).unwrap();
        let expected_sql = "SELECT [test].* FROM [test] WHERE [test].[id1] IN (@P1,@P2)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT [musti].* FROM [musti] ORDER BY [musti].[foo], [musti].[baz] ASC, [musti].[bar] DESC";
        let query = Select::from_table(Musti::table())
            .order_by(Musti::foo)
            .order_by(Musti::baz.ascend())
            .order_by(Musti::bar.descend());
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT [musti].[paw], [musti].[nose] FROM [cat].[musti]";
        let query = Select::from_table(Musti::table().database("cat"))
            .column(Musti::paw)
            .column(Musti::nose);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[derive(Entity)]
    #[tablename = "naukio"]
    struct Naukio {
        word: String,
        age: i32,
        paw: String,
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] = @P1",
            vec!["meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.equals("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] LIKE @P1",
            vec!["%meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.like("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] NOT LIKE @P1",
            vec!["%meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.not_like("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] LIKE @P1",
            vec!["meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.begins_with("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] NOT LIKE @P1",
            vec!["meow%"],
        );

        let query =
            Select::from_table(Naukio::table()).so_that(Naukio::word.not_begins_with("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] LIKE @P1",
            vec!["%meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.ends_into("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [naukio].[word] NOT LIKE @P1",
            vec!["%meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.not_ends_into("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[derive(Entity)]
    struct User {
        id: i32,
    }

    /*
    #[test]
    fn equality_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE CAST([users].[xmlField] AS NVARCHAR(MAX)) = @P1"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let query = Select::from_table(User::table())
            .so_that(Column::from(User::xml).equals(Value::xml("<cat>meow</cat>")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE @P1 = CAST([users].[xmlField] AS NVARCHAR(MAX))"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let value_expr: Expression = Value::xml("<cat>meow</cat>").into();
        let query = Select::from_table(User::table()).so_that(value_expr.equals(User::xml));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE CAST([users].[xmlField] AS NVARCHAR(MAX)) <> @P1"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let query = Select::from_table(User::table())
            .so_that(User::xml.not_equals(Value::xml("<cat>meow</cat>")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE @P1 <> CAST([users].[xmlField] AS NVARCHAR(MAX))"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let value_expr: Expression = Value::xml("<cat>meow</cat>").into();
        let query = Select::from_table(User::table()).so_that(value_expr.not_equals(User::xml));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }
    */

    #[test]
    fn test_select_and() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE ([naukio].[word] = @P1 AND [naukio].[age] < @P2 AND [naukio].[paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .and(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"));
        let query = Select::from_table(Naukio::table()).so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE ([naukio].[word] = @P1 AND ([naukio].[age] < @P2 AND [naukio].[paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .and(Naukio::age.less_than(10).and(Naukio::paw.equals("warm")));
        let query = Select::from_table(Naukio::table()).so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (([naukio].[word] = @P1 OR [naukio].[age] < @P2) AND [naukio].[paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .or(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"));

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([naukio].[word] = @P1 OR [naukio].[age] < @P2) AND [naukio].[paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .or(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"))
            .not();

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([naukio].[word] = @P1 OR [naukio].[age] < @P2) AND [naukio].[paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = ConditionTree::not(
            Naukio::word
                .equals("meow")
                .or(Naukio::age.less_than(10))
                .and(Naukio::paw.equals("warm")),
        );
        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[derive(Entity)]
    struct Post {
        user_id: i32,
        published: bool,
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] INNER JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table(User::table())
            .inner_join(Post::table().on(User::id.equals(Post::user_id)));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] INNER JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table(User::table()).inner_join(
            Post::table().on(User::id
                .equals(Post::user_id)
                .and(Post::published.equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] LEFT JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table(User::table())
            .left_join(Post::table().on(User::id.equals(Post::user_id)));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] LEFT JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table(User::table()).left_join(
            Post::table().on(User::id
                .equals(Post::user_id)
                .and(Post::published.equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[derive(Entity)]
    #[tablename = "meow"]
    struct Meow {
        bar: String,
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT [meow].[bar] AS [foo] FROM [meow]";
        let query = Select::from_table(Meow::table()).column(Meow::bar.alias("foo"));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "bar"]
    struct Bar {
        id: i32,
        foo: String,
    }

    #[test]
    fn test_limit_with_no_offset() {
        let expected_sql =
            "SELECT [bar].[foo] FROM [bar] ORDER BY [bar].[id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table(Bar::table())
            .column(Bar::foo)
            .order_by(Bar::id)
            .limit(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(0), Value::integer(10)], params);
    }

    #[test]
    fn test_offset_no_limit() {
        let expected_sql = "SELECT [bar].[foo] FROM [bar] ORDER BY [bar].[id] OFFSET @P1 ROWS";
        let query = Select::from_table(Bar::table())
            .column(Bar::foo)
            .order_by(Bar::id)
            .offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10)], params);
    }

    #[test]
    fn test_limit_with_offset() {
        let expected_sql =
            "SELECT [bar].[foo] FROM [bar] ORDER BY [bar].[id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table(Bar::table())
            .column(Bar::foo)
            .order_by(Bar::id)
            .limit(9)
            .offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10), Value::integer(9)], params);
    }

    #[test]
    fn test_limit_with_offset_no_given_order() {
        let expected_sql =
            "SELECT [bar].[foo] FROM [bar] ORDER BY 1 OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table(Bar::table())
            .column(Bar::foo)
            .limit(9)
            .offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10), Value::integer(9)], params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Mssql::build(Select::default().value(Value::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Mssql::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Mssql::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Mssql::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) =
            Mssql::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();

        assert_eq!("SELECT 0x010203", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Mssql::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());

        let (sql, params) = Mssql::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT 0", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) =
            Mssql::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_raw_json() {
        let (sql, params) =
            Mssql::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw()))
                .unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn test_raw_uuid() {
        let uuid = sqlx::types::Uuid::new_v4();
        let (sql, params) = Mssql::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(
            format!(
                "SELECT CONVERT(uniqueidentifier, N'{}')",
                uuid.to_hyphenated().to_string()
            ),
            sql
        );

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn test_raw_datetime() {
        let dt = sqlx::types::chrono::Utc::now();
        let (sql, params) = Mssql::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(
            format!("SELECT CONVERT(datetimeoffset, N'{}')", dt.to_rfc3339(),),
            sql
        );
        assert!(params.is_empty());
    }

    #[derive(Entity)]
    #[tablename = "foo"]
    struct Foo {
        foo: String,
        bar: String,
        wtf: String,
        lol: String,
        omg: String,
        baz: String,
    }

    #[test]
    fn test_single_insert() {
        let insert = Insert::single_into(Foo::table())
            .value(Foo::bar, "lol")
            .value(Foo::wtf, "meow");
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO [foo] ([foo].[bar],[foo].[wtf]) VALUES (@P1,@P2)",
            sql
        );
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_default() {
        let insert = Insert::single_into(Foo::table());
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] DEFAULT VALUES", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_returning_insert() {
        let insert = Insert::single_into(Foo::table()).value(Foo::bar, "lol");
        let (sql, params) = Mssql::build(Insert::from(insert).returning(vec![Foo::bar])).unwrap();

        assert_eq!("DECLARE @generated_keys table([bar] NVARCHAR(255)) INSERT INTO [foo] ([foo].[bar]) OUTPUT [Inserted].[bar] INTO @generated_keys VALUES (@P1) SELECT [t].[bar] FROM @generated_keys AS g INNER JOIN [foo] AS [t] ON [t].[bar] = [g].[bar] WHERE @@ROWCOUNT > 0", sql);

        assert_eq!(vec![Value::from("lol")], params);
    }

    #[test]
    fn test_multi_insert() {
        let insert = Insert::multi_into(Foo::table(), vec![Foo::bar, Foo::wtf])
            .values(vec!["lol", "meow"])
            .values(vec!["omg", "hey"]);

        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO [foo] ([foo].[bar],[foo].[wtf]) VALUES (@P1,@P2),(@P3,@P4)",
            sql
        );

        assert_eq!(
            vec![
                Value::from("lol"),
                Value::from("meow"),
                Value::from("omg"),
                Value::from("hey")
            ],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique() {
        let table = Foo::table().add_unique_index(Foo::bar);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::bar, "lol")
            .value(Foo::wtf, "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique_with_default() {
        let unique_column = Column::from(Foo::bar).default("purr");
        let table = Foo::table().add_unique_index(unique_column);

        let insert: Insert<'_> = Insert::single_into(table).value(Foo::wtf, "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON [foo].[bar] = @P2
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_single_insert_conflict_with_returning_clause() {
        let table = Foo::table().add_unique_index(Foo::bar);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::bar, "lol")
            .value(Foo::wtf, "meow")
            .into();

        let insert = insert
            .on_conflict(OnConflict::DoNothing)
            .returning(vec![Foo::bar, Foo::wtf]);

        let (sql, params) = Mssql::build(insert).unwrap();

        let expected_sql = indoc!(
            "
            DECLARE @generated_keys table([bar] NVARCHAR(255),[wtf] NVARCHAR(255))
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf])
            OUTPUT [Inserted].[bar],[Inserted].[wtf] INTO @generated_keys;
            SELECT [t].[bar],[t].[wtf] FROM @generated_keys AS g
            INNER JOIN [foo] AS [t]
            ON ([t].[bar] = [g].[bar] AND [t].[wtf] = [g].[wtf])
            WHERE @@ROWCOUNT > 0
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques() {
        let table = Foo::table()
            .add_unique_index(Foo::bar)
            .add_unique_index(Foo::wtf);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::bar, "lol")
            .value(Foo::wtf, "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques_with_default() {
        let unique_column = Column::from(Foo::bar).default("purr");

        let table = Foo::table()
            .add_unique_index(unique_column)
            .add_unique_index(Foo::wtf);

        let insert: Insert<'_> = Insert::single_into(table).value(Foo::wtf, "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn generated_unique_defaults_should_not_be_part_of_the_join_when_value_is_not_provided() {
        let unique_column = Column::from(Foo::bar).default("purr");
        let default_column = Column::from(Foo::lol).default(crate::ast::DefaultValue::Generated);

        let table = Foo::table()
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index(Foo::wtf);

        let insert: Insert<'_> = Insert::single_into(table).value(Foo::wtf, "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn with_generated_unique_defaults_the_value_should_be_part_of_the_join() {
        let unique_column = Column::from(Foo::bar).default("purr");
        let default_column = Column::from(Foo::lol).default(crate::ast::DefaultValue::Generated);

        let table = Foo::table()
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index(Foo::wtf);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::wtf, "meow")
            .value(Foo::lol, "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON ([foo].[bar] = @P3 OR [dual].[lol] = [foo].[lol] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);

        assert_eq!(
            vec![
                Value::from("meow"),
                Value::from("hiss"),
                Value::from("purr")
            ],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique() {
        let table = Foo::table().add_unique_index(vec![Foo::bar, Foo::wtf]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::bar, "lol")
            .value(Foo::wtf, "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique_with_default() {
        let bar = Column::from(Foo::bar).default("purr");
        let wtf = Column::from(Foo::wtf);

        let table = Foo::table().add_unique_index(vec![bar, wtf]);
        let insert: Insert<'_> = Insert::single_into(table).value(Foo::wtf, "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn one_generated_value_in_compound_unique_removes_the_whole_index_from_the_join() {
        let bar = Column::from(Foo::bar).default("purr");
        let wtf = Column::from(Foo::wtf);

        let omg = Column::from(Foo::omg).default(crate::ast::DefaultValue::Generated);
        let lol = Column::from(Foo::lol);

        let table = Foo::table()
            .add_unique_index(vec![bar, wtf])
            .add_unique_index(vec![omg, lol]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(Foo::wtf, "meow")
            .value(Foo::lol, "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON (([foo].[bar] = @P3 AND [dual].[wtf] = [foo].[wtf]) OR (1=0 AND [dual].[lol] = [foo].[lol]))
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(
            vec![
                Value::from("meow"),
                Value::from("hiss"),
                Value::from("purr")
            ],
            params
        );
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT [test].[bar] FROM [test]";
        let query = Select::from_table(TestEntity::table())
            .column(TestEntity::bar)
            .distinct();
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "test2"]
    struct Test2Entity {
        //
    }
    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT @P1 FROM [test2]), [test].[bar] FROM [test]";
        let query = Select::from_table(TestEntity::table())
            .value(Select::from_table(Test2Entity::table()).value(val!(1)))
            .column(TestEntity::bar)
            .distinct();

        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "baz"]
    struct Baz {
        a: String,
    }
    #[test]
    fn test_from() {
        let expected_sql = "SELECT [foo].*, [bar].[a] FROM [foo], (SELECT [a] FROM [baz]) AS [bar]";
        let query = Select::default()
            .and_from(Foo::table())
            .and_from(Table::from(Select::from_table(Baz::table()).column(Baz::a)).alias("bar"))
            .value(Foo::table().asterisk())
            .column(Baz::a);

        let (sql, _) = Mssql::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "A"]
    struct A {
        u: String,
        x: i32,
        y: i32,
        z: String,
    }

    #[test]
    fn test_cte_conversion_top_level_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default()
            .value(val!(1).alias("a"))
            .value(val!(2).alias("b"));
        let row = Row::from(vec![A::x, A::y]);
        let query = Select::from_table(A::table()).so_that(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2)], params);
    }

    #[test]
    fn test_cte_conversion_top_level_not_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] NOT IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default()
            .value(val!(1).alias("a"))
            .value(val!(2).alias("b"));
        let row = Row::from(vec![A::x, A::y]);
        let query = Select::from_table(A::table()).so_that(row.not_in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2)], params);
    }

    #[test]
    fn test_cte_conversion_in_a_tree_top_level() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3
            AND [A].[z] = @P4
            AND [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y]))"#
        )
        .replace('\n', " ");

        let inner = Select::default()
            .value(val!(1).alias("a"))
            .value(val!(2).alias("b"));
        let row = Row::from(vec![A::x, A::y]);

        let query = Select::from_table(A::table())
            .so_that(A::y.equals("bar"))
            .and_where(A::z.equals("foo"))
            .and_where(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::text("bar"),
                Value::text("foo")
            ],
            params
        );
    }

    #[test]
    fn test_cte_conversion_in_a_tree_nested() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3 OR ([A].[z] = @P4 AND [A].[x] IN
            (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])))"#
        )
        .replace('\n', " ");

        let inner = Select::default()
            .value(val!(1).alias("a"))
            .value(val!(2).alias("b"));
        let row = Row::from(vec![A::x, A::y]);

        let cond = A::y
            .equals("bar")
            .or(A::z.equals("foo").and(row.in_selection(inner)));

        let query = Select::from_table(A::table()).so_that(cond);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::text("bar"),
                Value::text("foo")
            ],
            params
        );
    }

    #[test]
    fn test_multiple_cte_conversions_in_the_ast() {
        let expected_sql = indoc!(
            r#"WITH
            [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b]),
            [cte_1] AS (SELECT @P3 AS [c], @P4 AS [d])
            SELECT [A].* FROM [A]
            WHERE ([A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])
            AND [A].[u] NOT IN (SELECT [c] FROM [cte_1] WHERE [d] = [A].[z]))"#
        )
        .replace('\n', " ");

        let cte_0 = Select::default()
            .value(val!(1).alias("a"))
            .value(val!(2).alias("b"));
        let cte_1 = Select::default()
            .value(val!(3).alias("c"))
            .value(val!(4).alias("d"));
        let row_0 = Row::from(vec![A::x, A::y]);
        let row_1 = Row::from(vec![A::u, A::z]);

        let query = Select::from_table(A::table())
            .so_that(row_0.in_selection(cte_0))
            .and_where(row_1.not_in_selection(cte_1));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4)
            ],
            params
        );
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into(Foo::table())
            .value(Foo::foo, "bar")
            .value(Foo::baz, default_value());

        let (sql, _) = Mssql::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO [foo] ([foo].[foo],[foo].[baz]) VALUES (@P1,DEFAULT)",
            sql
        );
    }

    #[derive(Entity)]
    #[tablename = "Toto"]
    struct Toto {}

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table =
            User::table().left_join(Post::table().alias("p").on(Post::user_id.equals(User::id)));
        let q = Select::from_table(joined_table).and_from(Toto::table());
        let (sql, _) = Mssql::build(q).unwrap();

        assert_eq!(
            "SELECT [users].*, [Toto].* FROM [users] LEFT JOIN [posts] AS [p] ON [p].[user_id] = [users].[id], [Toto]",
            sql
        );
    }
}
