use crate::{
    ast::*,
    visitors::{self, Visitor},
};
use std::fmt::{self, Write};

/// A visitor to generate queries for the postgres database.
///
/// The returned parameter values implement the `ToSql` trait from postgres and
/// can be used directly with the database.
#[cfg_attr(feature = "docs", doc(cfg(feature = "postgres")))]
pub struct Postgres<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Visitor<'a> for Postgres<'a> {
    const C_BACKTICK_OPEN: &'static str = "\"";
    const C_BACKTICK_CLOSE: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    #[tracing::instrument(name = "render_sql", skip(query))]
    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>,
    {
        let mut postgres = Postgres {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Postgres::visit_query(&mut postgres, query.into())?;

        Ok((postgres.query, postgres.parameters))
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> visitors::Result {
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn parameter_substitution(&mut self) -> visitors::Result {
        self.write("$")?;
        self.write(self.parameters.len())
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

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitors::Result {
        let res = match value {
            Value::Integer(i) => i.map(|i| self.write(i)),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Enum(e) => e.map(|e| self.write(e)),
            Value::Bytes(b) => b.map(|b| self.write(format!("E'{}'", hex::encode(b)))),
            Value::Boolean(b) => b.map(|b| self.write(b)),
            Value::Xml(cow) => cow.map(|cow| self.write(format!("'{}'", cow))),
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
            Value::Array(ary) => ary.map(|ary| {
                self.surround_with("'{", "}'", |ref mut s| {
                    let len = ary.len();

                    for (i, item) in ary.into_iter().enumerate() {
                        s.write(item)?;

                        if i < len - 1 {
                            s.write(",")?;
                        }
                    }

                    Ok(())
                })
            }),
            #[cfg(feature = "json-type")]
            Value::Json(j) => {
                j.map(|j| self.write(format!("'{}'", serde_json::to_string(&j).unwrap())))
            }
            #[cfg(feature = "bigdecimal-type")]
            Value::Numeric(r) => r.map(|r| self.write(r)),
            #[cfg(feature = "uuid-type")]
            Value::Uuid(uuid) => {
                uuid.map(|uuid| self.write(format!("'{}'", uuid.to_hyphenated().to_string())))
            }
            #[cfg(feature = "chrono-type")]
            Value::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            #[cfg(feature = "chrono-type")]
            Value::Date(date) => date.map(|date| self.write(format!("'{}'", date))),
            #[cfg(feature = "chrono-type")]
            Value::Time(time) => time.map(|time| self.write(format!("'{}'", time))),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitors::Result {
        self.write("INSERT ")?;

        if let Some(table) = insert.table {
            self.write("INTO ")?;
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
                            self.write(",")?;
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
                        self.write(",")?;
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
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        if let Some(OnConflict::DoNothing) = insert.on_conflict {
            self.write(" ON CONFLICT DO NOTHING")?;
        };

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values = returning.into_iter().map(|r| r.into()).collect();
                self.write(" RETURNING ")?;
                self.visit_columns(values)?;
            }
        };

        Ok(())
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> visitors::Result {
        self.write("ARRAY_TO_STRING")?;
        self.write("(")?;
        self.write("ARRAY_AGG")?;
        self.write("(")?;
        self.visit_expression(value)?;
        self.write(")")?;
        self.write("','")?;
        self.write(")")
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitors::Result {
        // LHS must be cast to json/xml-text if the right is a json/xml-text value and vice versa.
        let right_cast = match left {
            #[cfg(feature = "json-type")]
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            #[cfg(feature = "json-type")]
            _ if right.is_json_value() => "::jsonb",
            _ if right.is_xml_value() => "::text",
            _ => "",
        };

        self.visit_expression(left)?;
        self.write(left_cast)?;
        self.write(" = ")?;
        self.visit_expression(right)?;
        self.write(right_cast)?;

        Ok(())
    }

    fn visit_not_equals(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
    ) -> visitors::Result {
        // LHS must be cast to json/xml-text if the right is a json/xml-text value and vice versa.
        let right_cast = match left {
            #[cfg(feature = "json-type")]
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            #[cfg(feature = "json-type")]
            _ if right.is_json_value() => "::jsonb",
            _ if right.is_xml_value() => "::text",
            _ => "",
        };

        self.visit_expression(left)?;
        self.write(left_cast)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;
        self.write(right_cast)?;

        Ok(())
    }

    #[cfg(all(feature = "json-type", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitors::Result {
        match json_extract.path {
            #[cfg(feature = "mysql")]
            JsonPath::String(_) => {
                panic!("JSON path string notation is not supported for Postgres")
            }
            JsonPath::Array(json_path) => {
                self.write("(")?;
                self.visit_expression(*json_extract.column)?;

                if json_extract.extract_as_string {
                    self.write("#>>")?;
                } else {
                    self.write("#>")?;
                }

                // We use the `ARRAY[]::text[]` notation to better handle escaped character
                // The text protocol used when sending prepared statement doesn't seem to work well with escaped characters
                // when using the '{a, b, c}' string array notation.
                self.surround_with("ARRAY[", "]::text[]", |s| {
                    let len = json_path.len();
                    for (index, path) in json_path.into_iter().enumerate() {
                        s.visit_parameterized(Value::text(path))?;
                        if index < len - 1 {
                            s.write(", ")?;
                        }
                    }
                    Ok(())
                })?;
                self.write(")")
            }
        }
    }

    #[cfg(all(feature = "json-type", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_contains(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
        not: bool,
    ) -> visitors::Result {
        if not {
            self.write("( NOT ")?;
        }

        self.visit_expression(left)?;
        self.write(" @> ")?;
        self.visit_expression(right)?;

        if not {
            self.write(" )")?;
        }

        Ok(())
    }

    #[cfg(all(feature = "json-type", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_begins_with(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
        not: bool,
    ) -> visitors::Result {
        if not {
            self.write("( NOT ")?;
        }

        self.visit_expression(left)?;
        self.write("->0 = ")?;
        self.visit_expression(right)?;

        if not {
            self.write(" )")?;
        }

        Ok(())
    }

    #[cfg(all(feature = "json-type", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_array_ends_into(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
        not: bool,
    ) -> visitors::Result {
        if not {
            self.write("( NOT ")?;
        }

        self.visit_expression(left)?;
        self.write("->-1 = ")?;
        self.visit_expression(right)?;

        if not {
            self.write(" )")?;
        }

        Ok(())
    }

    #[cfg(all(feature = "json-type", any(feature = "postgres", feature = "mysql")))]
    fn visit_json_type_equals(
        &mut self,
        left: Expression<'a>,
        json_type: JsonType,
    ) -> visitors::Result {
        self.write("JSONB_TYPEOF")?;
        self.write("(")?;
        self.visit_expression(left)?;
        self.write(")")?;
        self.write(" = ")?;
        match json_type {
            JsonType::Array => self.visit_expression(Value::text("array").into()),
            JsonType::Boolean => self.visit_expression(Value::text("boolean").into()),
            JsonType::Number => self.visit_expression(Value::text("number").into()),
            JsonType::Object => self.visit_expression(Value::text("object").into()),
            JsonType::String => self.visit_expression(Value::text("string").into()),
            JsonType::Null => self.visit_expression(Value::text("null").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, visitors::*};
    use xiayu_derive::*;

    #[derive(Entity)]
    struct User {
        #[column(primary_key)]
        id: i32,
        foo: i32,
        #[cfg(feature = "json-type")]
        #[column(name = "jsonField")]
        json: serde_json::Value,
        #[column(name = "xmlField")]
        xml: String,
    }

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
    fn test_single_row_insert_default_values() {
        // let query = User::insert();
        let query = Insert::single_into(<User as Entity>::table());
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!("INSERT INTO \"users\" DEFAULT VALUES", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"users\".\"foo\") VALUES ($1)",
            vec![10],
        );
        let query = Insert::single_into(User::table()).value(User::foo, 10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn test_returning_insert() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"users\".\"foo\") VALUES ($1) RETURNING \"users\".\"foo\"",
            vec![10],
        );
        let query = Insert::single_into(User::table()).value(User::foo, 10);
        let (sql, params) =
            Postgres::build(Insert::from(query).returning(vec![User::foo])).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"users\".\"foo\") VALUES ($1), ($2)",
            vec![10, 11],
        );
        let query = Insert::multi_into(User::table(), vec![User::foo])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values(
            "SELECT \"users\".* FROM \"users\" LIMIT $1 OFFSET $2",
            vec![10, 2],
        );
        let query = Select::from_table(User::table()).limit(10).offset(2);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" OFFSET $1", vec![10]);
        let query = Select::from_table(User::table()).offset(10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1", vec![10]);
        let query = Select::from_table(User::table()).limit(10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[derive(Entity)]
    #[tablename = "test"]
    struct TestEntity {
        // #[column(primary_key)]
        bar: i32,
    }
    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT \"test\".\"bar\" FROM \"test\"";
        let query = Select::from_table(TestEntity::table())
            .column(TestEntity::bar)
            .distinct();
        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "test2"]
    struct SecondTestEntity {
        //
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql =
            "SELECT DISTINCT (SELECT $1 FROM \"test2\"), \"test\".\"bar\" FROM \"test\"";
        let query = Select::from_table(TestEntity::table())
            .value(Select::from_table(SecondTestEntity::table()).value(val!(1)))
            .column(TestEntity::bar)
            .distinct();

        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[derive(Entity)]
    #[tablename = "foo"]
    struct Foo {
        bar: String,
        foo: String,
    }

    #[derive(Entity)]
    #[tablename = "baz"]
    struct Baz {
        #[column(name = "a")]
        a_column: i32,
    }

    #[test]
    fn test_from() {
        let expected_sql =
            "SELECT \"foo\".*, \"bar\".\"a\" FROM \"foo\", (SELECT \"baz\".\"a\" FROM \"baz\") AS \"bar\"";
        let query = Select::default()
            .and_from(Foo::table())
            .and_from(
                Table::from(Select::from_table(Baz::table()).column(Baz::a_column)).alias("bar"),
            )
            .value(Foo::table().asterisk())
            .column(Baz::a_column);

        let (sql, _) = Postgres::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[cfg(feature = "json-type")]
    #[test]
    fn equality_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "users"."jsonField"::jsonb = $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table(User::table())
            .so_that(User::json.equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-type")]
    #[test]
    fn equality_with_a_lhs_json_value() {
        // A bit artificial, but checks if the ::jsonb casting is done correctly on the right side as well.
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 = "users"."jsonField"::jsonb"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let value_expr: Expression = Value::json(serde_json::json!({"a":"b"})).into();
        let query = Select::from_table(User::table()).so_that(value_expr.equals(User::json));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-type")]
    #[test]
    fn difference_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "users"."jsonField"::jsonb <> $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table(User::table())
            .so_that(User::json.not_equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-type")]
    #[test]
    fn difference_with_a_lhs_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 <> "users"."jsonField"::jsonb"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let value_expr: Expression = Value::json(serde_json::json!({"a":"b"})).into();
        let query = Select::from_table(User::table()).so_that(value_expr.not_equals(User::json));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "users"."xmlField"::text = $1"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let query = Select::from_table(User::table())
            .so_that(User::xml.equals(Value::xml("<salad>wurst</salad>")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 = "users"."xmlField"::text"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let value_expr: Expression = Value::xml("<salad>wurst</salad>").into();
        let query = Select::from_table(User::table()).so_that(value_expr.equals(User::xml));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "users"."xmlField"::text <> $1"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let query = Select::from_table(User::table())
            .so_that(User::xml.not_equals(Value::xml("<salad>wurst</salad>")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 <> "users"."xmlField"::text"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let value_expr: Expression = Value::xml("<salad>wurst</salad>").into();
        let query = Select::from_table(User::table()).so_that(value_expr.not_equals(User::xml));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) =
            Postgres::build(Select::default().value(Value::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Postgres::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Postgres::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Postgres::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) =
            Postgres::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT E'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Postgres::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Postgres::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) =
            Postgres::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "json-type")]
    fn test_raw_json() {
        let (sql, params) =
            Postgres::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw()))
                .unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid-type")]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Postgres::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(
            format!("SELECT '{}'", uuid.to_hyphenated().to_string()),
            sql
        );

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono-type")]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Postgres::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_comparator() {
        let (sql, _) = Postgres::build(
            Select::from_table(Foo::table()).so_that(Foo::bar.compare_raw("ILIKE", "baz%")),
        )
        .unwrap();

        assert_eq!(
            r#"SELECT "foo".* FROM "foo" WHERE "foo"."bar" ILIKE $1"#,
            sql
        );
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into(Foo::table())
            .value(Foo::foo, "bar")
            .value(Foo::bar, default_value());

        let (sql, _) = Postgres::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO \"foo\" (\"foo\".\"foo\",\"foo\".\"bar\") VALUES ($1,DEFAULT)",
            sql
        );
    }

    #[test]
    fn join_is_inserted_positionally() {
        #[derive(Entity)]
        #[tablename = "User"]
        struct User {
            #[column(primary_key)]
            id: i32,
        }
        #[derive(Entity)]
        #[tablename = "Post"]
        struct Post {
            #[column(primary_key)]
            id: i32,
            #[column(name = "userId")]
            user_id: i32,
        }
        #[derive(Entity)]
        #[tablename = "Toto"]
        struct Toto {
            #[column(primary_key)]
            id: i32,
        }
        let joined_table =
            User::table().left_join(Post::table().alias("p").on(Post::user_id.equals(User::id)));
        let q = Select::from_table(joined_table).and_from(Toto::table());
        let (sql, _) = Postgres::build(q).unwrap();

        assert_eq!("SELECT \"User\".*, \"Toto\".* FROM \"User\" LEFT JOIN \"Post\" AS \"p\" ON \"p\".\"userId\" = \"User\".\"id\", \"Toto\"", sql);
    }
}
