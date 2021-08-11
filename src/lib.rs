#![cfg_attr(feature = "docs", feature(doc_cfg))]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate xiayu_derive;

#[cfg(not(any(
    feature = "sqlite",
    feature = "postgresql",
    feature = "mysql",
    feature = "mssql"
)))]
compile_error!("one of 'sqlite', 'postgresql', 'mysql' or 'mssql' features must be enabled");

#[cfg(feature = "bigdecimal")]
extern crate bigdecimal as bigdecimal;

#[macro_use]
mod macros;
#[macro_use]
pub mod visitors;
pub mod ast;
pub mod error;
#[cfg(feature = "serde")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "serde")))]
pub mod serde;

pub type Result<T> = std::result::Result<T, error::Error>;

pub mod prelude {
    use std::marker::PhantomData;

    pub use xiayu_derive::Entity;

    pub use super::ast::*;
    pub use super::Result;

    /// select().where(Entity::last_modified == now())
    pub trait Entity {
        type PrimaryKey;
        const COLUMNS: &'static [Column<'static>];
        fn primary_key() -> <Self as Entity>::PrimaryKey;
        fn tablename() -> &'static str;
        fn columns() -> &'static [Column<'static>];
        fn table() -> Table<'static>;
    }

    pub trait EntityInstanced: Entity {
        fn primary_key(&self) -> <Self as Entity>::PrimaryKey {
            <Self as Entity>::primary_key()
        }

        fn tablename(&self) -> &'static str {
            <Self as Entity>::tablename()
        }

        fn columns(&self) -> &'static [Column<'static>] {
            <Self as Entity>::columns()
        }

        fn table(&self) -> Table<'static> {
            <Self as Entity>::table()
        }
    }

    impl<T> EntityInstanced for T where T: Entity {}

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
        default: Option<T>,
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
            default: Option<T>,
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
    }

    impl<'a, T> From<ColumnOptions<T>> for Column<'a> {
        fn from(options: ColumnOptions<T>) -> Self {
            options.column()
        }
    }

    impl<'a, T> From<&ColumnOptions<T>> for Column<'a> {
        fn from(options: &ColumnOptions<T>) -> Self {
            options.column()
        }
    }

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
