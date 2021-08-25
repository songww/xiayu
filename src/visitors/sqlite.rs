use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitors::{self, Visitor},
};

use std::fmt::{self, Write};

/// A visitor to generate queries for the SQLite database.
///
/// The returned parameter values implement the `ToSql` trait from rusqlite and
/// can be used directly with the database.
#[cfg_attr(feature = "docs", doc(cfg(feature = "sqlite")))]
pub struct Sqlite<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Default for Sqlite<'a> {
    fn default() -> Self {
        Sqlite {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        }
    }
}

impl<'a> Visitor<'a> for Sqlite<'a> {
    const C_BACKTICK_OPEN: &'static str = "`";
    const C_BACKTICK_CLOSE: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    #[tracing::instrument(name = "render_sql", skip(query))]
    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>,
    {
        let mut sqlite = Sqlite {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Sqlite::visit_query(&mut sqlite, query.into())?;

        Ok((sqlite.query, sqlite.parameters))
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> visitors::Result {
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitors::Result {
        let res = match value {
            Value::Integer(i) => i.map(|i| self.write(i)),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Enum(e) => e.map(|e| self.write(e)),
            Value::Bytes(b) => b.map(|b| self.write(format!("x'{}'", hex::encode(b)))),
            Value::Boolean(b) => b.map(|b| self.write(b)),
            Value::Char(c) => c.map(|c| self.write(format!("'{}'", c))),
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
            Value::Array(_) => {
                let msg = "Arrays are not supported in SQLite.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(builder.build());
            }
            #[cfg(feature = "json")]
            Value::Json(j) => match j {
                crate::ast::Json::JsonValue(Some(ref v)) => {
                    let s = serde_json::to_string(&v)?;
                    Some(self.write(format!("'{}'", s)))
                }
                crate::ast::Json::JsonRawValue(Some(ref v)) => {
                    let s = serde_json::to_string(&**v)?;
                    Some(self.write(format!("'{}'", s)))
                }
                _ => None,
            },
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(r) => r.map(|r| self.write(r)),
            #[cfg(feature = "uuid")]
            Value::Uuid(uuid) => {
                uuid.map(|uuid| self.write(format!("'{}'", uuid.to_hyphenated().to_string())))
            }
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            #[cfg(feature = "chrono")]
            Value::Date(date) => date.map(|date| self.write(format!("'{}'", date))),
            #[cfg(feature = "chrono")]
            Value::Time(time) => time.map(|time| self.write(format!("'{}'", time))),
            Value::Xml(cow) => cow.map(|cow| self.write(format!("'{}'", cow))),
            _ => todo!(),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitors::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT OR IGNORE")?,
            None => self.write("INSERT")?,
        };

        if let Some(table) = insert.table {
            self.write(" INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" DEFAULT VALUES")?;
                } else {
                    let columns = insert.columns.len();

                    self.write(" (")?;
                    for (i, c) in insert.columns.into_iter().enumerate() {
                        self.visit_column(c)?;

                        if i < (columns - 1) {
                            self.write(", ")?;
                        }
                    }

                    self.write(")")?;
                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                let columns = insert.columns.len();

                self.write(" (")?;
                for (i, c) in insert.columns.into_iter().enumerate() {
                    self.visit_column(c)?;

                    if i < (columns - 1) {
                        self.write(", ")?;
                    }
                }
                self.write(")")?;

                self.write(" VALUES ")?;
                let values_len = values.len();

                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(", ")?;
                    }
                }
            }
            expr => self.visit_expression(expr)?,
        }

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values_len = returning.len();
                self.write(" RETURNING ")?;

                for (i, column) in returning.into_iter().enumerate() {
                    self.write(&column.name)?;

                    if i < (values_len - 1) {
                        self.write(", ")?;
                    }
                }
            }
        };

        Ok(())
    }

    fn parameter_substitution(&mut self) -> visitors::Result {
        self.write("?")
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<Value<'a>>,
        offset: Option<Value<'a>>,
    ) -> visitors::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(Value::from(-1))?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (Some(limit), None) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> visitors::Result {
        self.write("GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_values(&mut self, values: Values<'a>) -> visitors::Result {
        self.surround_with("(VALUES ", ")", |ref mut s| {
            let len = values.len();
            for (i, row) in values.into_iter().enumerate() {
                s.visit_row(row)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }
            Ok(())
        })
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_extract(&mut self, _json_extract: JsonExtract<'a>) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on SQLite")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_contains(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on SQLite")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_begins_with(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on SQLite")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_ends_into(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("JSON filtering is not yet supported on SQLite")
    }

    #[cfg(all(feature = "json", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_type_equals(
        &mut self,
        _left: Expression<'a>,
        _json_type: JsonType,
    ) -> visitors::Result {
        unimplemented!("JSON_TYPE is not yet supported on SQLite")
    }

    #[cfg(feature = "postgres")]
    fn visit_text_search(
        &mut self,
        _text_search: crate::prelude::TextSearch<'a>,
    ) -> visitors::Result {
        unimplemented!("Full-text search is not yet supported on SQLite")
    }

    #[cfg(feature = "postgres")]
    fn visit_matches(
        &mut self,
        _left: Expression<'a>,
        _right: std::borrow::Cow<'a, str>,
        _not: bool,
    ) -> visitors::Result {
        unimplemented!("Full-text search is not yet supported on SQLite")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::{val, visitors::*};

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
        let expected = expected_values("SELECT ?", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT ? AS `test`", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT ? AS `test`";
        let query = Select::default().value(val!(Value::Text(None)).alias("test"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::Text(None)], params);
    }

    #[derive(Entity)]
    #[tablename = "musti"]
    struct Musti {
        foo: i32,
        baz: i32,
        bar: i32,
        paw: i32,
        nose: i32,
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT `musti`.* FROM `musti`";
        let query = Select::from_table(Musti::table());
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[derive(Entity)]
    #[tablename = "test"]
    struct TestEntity {
        id1: i32,
        id2: i32,
        bar: i32,
    }

    #[test]
    fn test_select_from_values() {
        use crate::values;

        let expected_sql = "SELECT `vals`.* FROM (VALUES (?,?),(?,?)) AS `vals`";
        let values = Table::from(values!((1, 2), (3, 4))).alias("vals");
        let query = Select::from_table(values);
        let (sql, params) = Sqlite::build(query).unwrap();

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
    fn test_in_values() {
        use crate::values;

        let expected_sql =
            "SELECT `test`.* FROM `test` WHERE (`test`.`id1`,`test`.`id2`) IN (VALUES (?,?),(?,?))";
        let query = Select::from_table(TestEntity::table()).so_that(
            Row::from((TestEntity::id1, TestEntity::id2)).in_selection(values!((1, 2), (3, 4))),
        );

        let (sql, params) = Sqlite::build(query).unwrap();

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
        let (sql, params) = Sqlite::build(query).unwrap();
        let expected_sql = "SELECT `test`.* FROM `test` WHERE `test`.`id1` IN (?,?)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT `musti`.* FROM `musti` ORDER BY `musti`.`foo`, `musti`.`baz` ASC, `musti`.`bar` DESC";
        let query = Select::from_table(Musti::table())
            .order_by(Musti::foo)
            .order_by(Musti::baz.ascend())
            .order_by(Musti::bar.descend());
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT `musti`.`paw`, `musti`.`nose` FROM `cat`.`musti`";
        let query = Select::from_table(Musti::table().database("cat"))
            .column(Musti::paw)
            .column(Musti::nose);
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[derive(Entity)]
    #[tablename = "naukio"]
    struct Naukio {
        word: String,
        paw: String,
        age: i32,
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` = ?",
            vec!["meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.equals("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` LIKE ?",
            vec!["%meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.like("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` NOT LIKE ?",
            vec!["%meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.not_like("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` LIKE ?",
            vec!["meow%"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.begins_with("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` NOT LIKE ?",
            vec!["meow%"],
        );

        let query =
            Select::from_table(Naukio::table()).so_that(Naukio::word.not_begins_with("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` LIKE ?",
            vec!["%meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.ends_into("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `naukio`.`word` NOT LIKE ?",
            vec!["%meow"],
        );

        let query = Select::from_table(Naukio::table()).so_that(Naukio::word.not_ends_into("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (`naukio`.`word` = ? AND `naukio`.`age` < ? AND `naukio`.`paw` = ?)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .and(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"));

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (`naukio`.`word` = ? AND (`naukio`.`age` < ? AND `naukio`.`paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .and(Naukio::age.less_than(10).and(Naukio::paw.equals("warm")));

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE ((`naukio`.`word` = ? OR `naukio`.`age` < ?) AND `naukio`.`paw` = ?)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .or(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"));

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`naukio`.`word` = ? OR `naukio`.`age` < ?) AND `naukio`.`paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = Naukio::word
            .equals("meow")
            .or(Naukio::age.less_than(10))
            .and(Naukio::paw.equals("warm"))
            .not();

        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`naukio`.`word` = ? OR `naukio`.`age` < ?) AND `naukio`.`paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = ConditionTree::not(
            Naukio::word
                .equals("meow")
                .or(Naukio::age.less_than(10))
                .and(Naukio::paw.equals("warm")),
        );
        let query = Select::from_table(Naukio::table()).so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[derive(Entity)]
    struct User {
        id: i32,
    }

    #[derive(Entity)]
    struct Post {
        user_id: i32,
        published: bool,
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table(User::table())
            .inner_join(Post::table().on(User::id.equals(Post::user_id)));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` INNER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table(User::table()).inner_join(
            Post::table().on(User::id
                .equals(Post::user_id)
                .and(Post::published.equals(true))),
        );

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table(User::table())
            .left_join(Post::table().on(User::id.equals(Post::user_id)));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table(User::table()).left_join(
            Post::table().on(User::id
                .equals(Post::user_id)
                .and(Post::published.equals(true))),
        );

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[derive(Entity)]
    #[tablename = "meow"]
    struct Meow {
        bar: i32,
    }
    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT `meow`.`bar` AS `foo` FROM `meow`";
        let query = Select::from_table(Meow::table()).column(Meow::bar.alias("foo"));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT `test`.`bar` FROM `test`";
        let query = Select::from_table(TestEntity::table())
            .column(TestEntity::bar)
            .distinct();
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "test2"]
    struct Test2Entity {}

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT ? FROM `test2`), `test`.`bar` FROM `test`";
        let query = Select::from_table(TestEntity::table())
            .value(Select::from_table(Test2Entity::table()).value(val!(1)))
            .column(TestEntity::bar)
            .distinct();

        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "foo"]
    struct Foo {
        foo: String,
        baz: String,
    }

    #[derive(Entity)]
    #[tablename = "baz"]
    struct Baz {
        a: i32,
    }

    #[test]
    fn test_from() {
        let expected_sql =
            "SELECT `foo`.*, `bar`.`a` FROM `foo`, (SELECT `baz`.`a` FROM `baz`) AS `bar`";
        let query = Select::default()
            .and_from(Foo::table())
            .and_from(Table::from(Select::from_table(Baz::table()).column(Baz::a)).alias("bar"))
            .value(Foo::table().asterisk())
            .column(Baz::a);

        let (sql, _) = Sqlite::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) =
            Sqlite::build(Select::default().value(Value::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Sqlite::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Sqlite::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Sqlite::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) =
            Sqlite::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT x'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Sqlite::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Sqlite::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) =
            Sqlite::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_raw_json() {
        let (sql, params) =
            Sqlite::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw()))
                .unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Sqlite::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(
            format!("SELECT '{}'", uuid.to_hyphenated().to_string()),
            sql
        );

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Sqlite::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into(Foo::table())
            .value(Foo::foo, "bar")
            .value(Foo::baz, default_value());

        let (sql, _) = Sqlite::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO `foo` (`foo`.`foo`, `foo`.`baz`) VALUES (?,DEFAULT)",
            sql
        );
    }

    #[derive(Entity)]
    #[tablename = "Toto"]
    struct Toto {}

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table = Table::from(User::table())
            .left_join(Post::table().alias("p").on(Post::user_id.equals(User::id)));
        let q = Select::from_table(joined_table).and_from(Toto::table());
        let (sql, _) = Sqlite::build(q).unwrap();

        assert_eq!(
            "SELECT `users`.*, `Toto`.* FROM `users` LEFT JOIN `posts` AS `p` ON `p`.`user_id` = `user`.`id`, `Toto`",
            sql
        );
    }
}
