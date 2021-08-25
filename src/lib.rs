#![feature(const_panic)]
#![cfg_attr(feature = "docs", feature(doc_cfg))]

#[cfg(not(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "mssql"
)))]
compile_error!("one of 'sqlite', 'postgres', 'mysql' or 'mssql' features must be enabled");

#[macro_use]
mod macros;
#[macro_use]
pub mod visitors;
pub mod ast;
pub mod databases;
pub mod error;

pub type Result<T> = std::result::Result<T, error::Error>;

pub mod prelude {
    use std::future::Future;
    use std::marker::PhantomData;

    use sqlx::Database;
    use sqlx::Executor;
    pub use xiayu_derive::Entity;

    pub use crate::databases::{DeleteRequest, FetchRequest, SaveRequest};

    pub use super::ast::*;
    pub use super::Result;

    #[derive(Clone, Debug)]
    pub struct DefaultValue<T>(fn() -> T);

    impl<T> DefaultValue<T> {
        fn get(&self) -> T {
            (self.0)()
        }
    }

    /// select().where(Entity::last_modified == now())
    pub trait Entity {
        const COLUMNS: &'static [Column<'static>];
        fn tablename() -> &'static str;
        fn columns() -> &'static [Column<'static>];
        fn table() -> Table<'static>;

        /*
        fn select<'a, E>() -> Select<'a>
        where
            E: Entity,
        {
            Select::from_table(Self::table()).columns(E::columns())
        }
        */
    }

    pub trait EntityInstantiated: Entity {
        fn tablename(&self) -> &'static str {
            <Self as Entity>::tablename()
        }

        fn columns(&self) -> &'static [Column<'static>] {
            <Self as Entity>::columns()
        }
    }

    impl<T> EntityInstantiated for T where T: Entity {}

    pub trait HasPrimaryKey: Entity {
        type PrimaryKey;
        type PrimaryKeyValueType;
        fn primary_key() -> <Self as HasPrimaryKey>::PrimaryKey;
        fn pk(&self) -> <Self as HasPrimaryKey>::PrimaryKeyValueType;
        fn get<DB: sqlx::Database>(pk: Self::PrimaryKeyValueType) -> FetchRequest<Self, DB>
        where
            Self: for<'r> sqlx::FromRow<'r, <DB as sqlx::Database>::Row> + Sized;
        fn delete<'e, DB: sqlx::Database>(&'e mut self) -> DeleteRequest<'e, Self, DB>
        where
            Self: Sized;
        fn save<'e, DB: sqlx::Database>(&'e mut self) -> SaveRequest<'e, Self, DB>
        where
            Self: Sized;
    }

    #[derive(Clone)]
    pub struct ColumnOptions<T> {
        name: &'static str,
        tablename: &'static str,
        /// Set up "auto increment" semantics for an integer primary key column.
        /// The default value is the string "auto" which indicates that a single-column primary key that is of an INTEGER type with no stated client-side or python-side defaults should receive auto increment semantics automatically; all other varieties of primary key columns will not.
        /// This includes that DDL such as PostgreSQL SERIAL or MySQL AUTO_INCREMENT will be emitted for this column during a table create, as well as that the column is assumed to generate new integer primary key values when an INSERT statement invokes which will be retrieved by the dialect.
        /// When used in conjunction with Identity on a dialect that supports it, this parameter has no effect.
        primary_key: bool,
        autoincrement: bool,
        /// Optional string that will render an SQL comment on table creation.
        comment: Option<&'static str>,
        unique: bool,
        foreign_key: Option<&'static str>,
        /// The name of this column as represented in the database. This argument may be the first positional argument, or specified via keyword.
        length: Option<usize>,
        quote: bool,
        default: Option<DefaultValue<T>>,
        _phantom: PhantomData<T>,
        /*
        onupdate: Option<Arc<Box<dyn Fn() -> T>>>,
        server_default: Option<String>,
        server_onupdate: Option<String>,
        */
    }

    impl<T> ColumnOptions<T> {
        pub const fn new(
            name: &'static str,
            tablename: &'static str,
            primary_key: bool,
            autoincrement: bool,
            foreign_key: Option<&'static str>,
            comment: Option<&'static str>,
            unique: bool,
            length: Option<usize>,
            quote: bool,
            default: Option<DefaultValue<T>>,
        ) -> Self {
            Self {
                name,
                tablename,
                primary_key,
                autoincrement,
                foreign_key,
                comment,
                unique,
                length,
                quote,
                default,
                _phantom: PhantomData,
            }
        }

        pub const fn column(&self) -> Column<'static> {
            Column {
                name: std::borrow::Cow::Borrowed(self.name),
                table: Some(self.table()),
                alias: None,
                default: None,
                type_family: None,
            }
        }

        pub const fn table(&self) -> Table<'static> {
            Table {
                typ: TableType::Table(std::borrow::Cow::Borrowed(self.tablename)),
                alias: None,
                database: None,
                index_definitions: Vec::new(),
            }
        }

        pub fn c(&self) -> Column<'static> {
            self.column()
        }

        pub fn t(&self) -> Table<'static> {
            self.table()
        }

        // pub create_table() -> CreateTable;
    }

    impl<'a, T> From<ColumnOptions<T>> for Column<'a> {
        fn from(options: ColumnOptions<T>) -> Self {
            options.c()
        }
    }

    impl<'a, T> From<&ColumnOptions<T>> for Column<'a> {
        fn from(options: &ColumnOptions<T>) -> Self {
            options.c()
        }
    }

    impl<'a, T> From<ColumnOptions<T>> for Expression<'a> {
        fn from(col: ColumnOptions<T>) -> Self {
            Expression {
                kind: ExpressionKind::Column(Box::new(col.column())),
                alias: None,
            }
        }
    }

    impl<'a, T> From<&ColumnOptions<T>> for Expression<'a> {
        fn from(col: &ColumnOptions<T>) -> Self {
            Expression {
                kind: ExpressionKind::Column(Box::new(col.column())),
                alias: None,
            }
        }
    }

    impl<'a, T> Aliasable<'a> for ColumnOptions<T> {
        type Target = Column<'a>;

        fn alias<A>(self, alias: A) -> Self::Target
        where
            A: Into<std::borrow::Cow<'a, str>>,
        {
            let mut target = self.column();
            target.alias = Some(alias.into());
            target
        }
    }

    impl<'a, T> Aliasable<'a> for T
    where
        T: Entity,
    {
        type Target = Table<'a>;

        fn alias<A>(self, alias: A) -> Self::Target
        where
            A: Into<::std::borrow::Cow<'a, str>>,
        {
            let mut table = T::table();
            table.alias.replace(alias.into());
            table
        }
    }

    impl<'a, T> IntoOrderDefinition<'a> for ColumnOptions<T> {
        fn into_order_definition(self) -> OrderDefinition<'a> {
            (self.column().into(), None)
        }
    }

    impl<'a, T> Orderable<'a> for ColumnOptions<T> {
        fn order(self, order: Option<Order>) -> OrderDefinition<'a> {
            (self.column().into(), order)
        }
    }

    /*
    impl<'a, T> PartialEq for ColumnOptions<T> {
        fn eq(&self, other: &ColumnOptions<T>) -> bool {
            self.name == other.name && self.table == other.table
        }
    }
    */

    /*
    impl<'a> ::xiayu::prelude::Selectable<'a> for #ident {
        fn select<C: AsRef<&[Column]>>(columns: C) {}
        //
    }
    */

    pub struct Many<T>
    where
        T: Entity,
    {
        _phantom: PhantomData<T>,
    }

    pub struct Relationship<T>
    where
        T: Entity,
    {
        _phantom: PhantomData<T>,
    }
}
