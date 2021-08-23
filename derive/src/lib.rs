#[macro_use]
extern crate darling;

use darling::{FromDeriveInput, FromMeta};
use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

#[derive(FromDeriveInput)]
#[darling(attributes(entity), supports(struct_named))]
struct EntityDefinition {
    /// The struct ident.
    ident: syn::Ident,

    /// The type's generics. You'll need these any time your trait is expected
    /// to work with types that declare generics.
    generics: syn::Generics,

    /// Receives the body of the struct or enum. We don't care about
    /// struct fields because we previously told darling we only accept structs.
    data: darling::ast::Data<(), ColumnOptions>,

    /// The visibility of the passed-in type
    vis: syn::Visibility,

    /// The forwarded attributes from the passed in type. These are controlled using the
    /// forward_attrs attribute.
    attrs: Vec<syn::Attribute>,
}

#[derive(Clone, Debug, Default, FromMeta)]
struct Tablename(String);

#[derive(Clone, Debug, Default)]
struct EntityOptions {
    tablename: Option<String>,
    indexes: Vec<IndexMeta>,
}

#[derive(Clone, Debug, FromMeta)]
struct IndexMeta {
    pub name: String,
    pub columns: String,
    #[darling(default)]
    pub unique: bool,
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(column))]
struct ColumnOptions {
    #[darling(default)]
    primary_key: darling::util::Flag,
    /// Set up "auto increment" semantics for an integer primary key column.
    /// The default value is the string "auto" which indicates that a single-column primary key that is of an INTEGER type with no stated client-side or python-side defaults should receive auto increment semantics automatically; all other varieties of primary key columns will not.
    /// This includes that DDL such as PostgreSQL SERIAL or MySQL AUTO_INCREMENT will be emitted for this column during a table create, as well as that the column is assumed to generate new integer primary key values when an INSERT statement invokes which will be retrieved by the dialect.
    /// When used in conjunction with Identity on a dialect that supports it, this parameter has no effect.
    #[darling(default)]
    autoincrement: darling::util::Flag,
    /// Optional string that will render an SQL comment on table creation.
    #[darling(default)]
    comment: Option<syn::LitStr>,
    #[darling(default)]
    unique: bool,
    /// The name of this column as represented in the database. This argument may be the first positional argument, or specified via keyword.
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    length: Option<usize>,
    #[darling(default)]
    default: Option<syn::Lit>,
    #[darling(default)]
    onupdate: Option<String>,
    #[darling(default)]
    foreign_key: Option<syn::LitStr>,
    #[darling(default)]
    server_default: Option<syn::Lit>,
    #[darling(default)]
    server_onupdate: Option<String>,
    /// Force quoting of this column’s name on or off, corresponding to true or false.
    /// When left at its default of None, the column identifier will be quoted according to whether the name is case sensitive (identifiers with at least one upper case character are treated as case sensitive), or if it’s a reserved word.
    /// This flag is only needed to force quoting of a reserved word which is not known by the SQLAlchemy dialect.
    #[darling(default)]
    quote: bool,

    /// Get the ident of the field. For fields in tuple or newtype structs or
    /// enum bodies, this can be `None`.
    ident: Option<syn::Ident>,

    /// This magic field name pulls the type from the input.
    ty: syn::Type,

    /// The visibility of the passed-in type
    vis: syn::Visibility,

    /// The forwarded attributes from the passed in type. These are controlled using the
    /// forward_attrs attribute.
    attrs: Vec<syn::Attribute>,
}

macro_rules! quote_optional {
    ($expr:expr) => {
        match $expr {
            Some(value) => {
                quote! { Some(#value) }
            }
            None => {
                quote! { None }
            }
        }
    };
}

#[proc_macro_derive(Entity, attributes(tablename, column))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let mut entity_options = EntityOptions::default();
    for attr in input.attrs.iter() {
        let meta = match attr.parse_meta() {
            Ok(meta) => meta,
            Err(err) => return err.into_compile_error().into(),
        };
        if let Ok(Tablename(tablename)) = Tablename::from_meta(&meta) {
            entity_options.tablename.replace(tablename);
            continue;
        }

        if let Ok(index) = IndexMeta::from_meta(&meta) {
            entity_options.indexes.push(index);
            continue;
        }
        eprintln!("Invalid attr {:?}", attr);
    }
    let entity_def = match EntityDefinition::from_derive_input(&input) {
        Ok(def) => def,
        Err(err) => return err.write_errors().into(),
    };
    let ident = entity_def.ident;
    let tablename = entity_options
        .tablename
        .clone()
        .unwrap_or_else(|| ident.to_string().to_table_case());

    let mut primary_key_type = None;
    let mut primary_key_column = None;
    let mut primary_key_value_type = None;
    let mut primary_key_column_name = None;
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut column_options = Vec::new();

    let mut tokens = TokenStream2::new();

    let found_crate =
        proc_macro_crate::crate_name("xiayu").expect("xiayu is not present in `Cargo.toml`");

    let namespace = match found_crate {
        proc_macro_crate::FoundCrate::Itself => quote!(self),
        proc_macro_crate::FoundCrate::Name(name) => {
            let import = format_ident!("{}", &name);
            quote!( #import::prelude )
        }
    };

    let generics = &entity_def.generics;

    let (lifetime, provided) = generics
        .lifetimes()
        .next()
        .map(|def| (def.lifetime.clone(), false))
        .unwrap_or_else(|| {
            (
                syn::Lifetime::new("'a", proc_macro2::Span::call_site()),
                true,
            )
        });

    let (_, ty_generics, _) = generics.split_for_impl();

    let mut generics = generics.clone();
    generics.params.insert(0, syn::parse_quote!(R: ::sqlx::Row));

    if provided {
        generics.params.insert(0, syn::parse_quote!(#lifetime));
    }

    let where_clause = generics.make_where_clause();
    let predicates = &mut where_clause.predicates;

    predicates.push(syn::parse_quote!(&#lifetime ::std::primitive::str: ::sqlx::ColumnIndex<R>));

    let mut reads: Vec<syn::Stmt> = Vec::new();
    if let darling::ast::Data::Struct(darling::ast::Fields { fields, .. }) = entity_def.data {
        for field in fields.into_iter() {
            let ty = field.ty;
            types.push(ty.clone());
            let name = field
                .ident
                .as_ref()
                .map(|i| i.to_string().trim_start_matches("r#").to_owned())
                .unwrap();
            let column_name = field.name.unwrap_or(name.to_string());
            let is_primary_key = field.primary_key.is_some();
            let autoincrement = field.autoincrement.is_some();
            let comment = quote_optional!(field.comment.map(|v| { v.value().to_string() }));
            let foreign_key = quote_optional!(field.foreign_key.map(|v| v.value().to_string()));
            let unique = field.unique;
            let length = quote_optional!(field.length);
            let quote_name = field.quote;
            let default = quote_optional!(field.default.clone());
            let onupdate = quote_optional!(field.onupdate);
            let server_default = quote_optional!(field.server_default);
            let server_onupdate = quote_optional!(field.server_onupdate);
            let column = quote! {
                ColumnOptions::new(
                    /* name: */ #column_name,
                    /* tablename: */ #tablename,
                    /* primary_key: */ #is_primary_key,
                    /* autoincrement: */ #autoincrement,
                    /* foreign_key: */ #foreign_key,
                    /* comment: */ #comment,
                    /* unique: */ #unique,
                    /* length: */ #length,
                    /* quote: */ #quote_name,
                    /* default: */ #default,
                    /* onupdate: #onupdate,
                    server_default: #server_default,
                    server_onupdate: #server_onupdate,
                    */
                )
            };
            if is_primary_key {
                primary_key_type = Some(quote! { #namespace::ColumnOptions<#ty> });
                primary_key_value_type = Some(quote! { #ty });
                // println!("primary_key_definition: {:?}", column.clone().to_string());
                primary_key_column = Some(column.clone());
                primary_key_column_name = Some(format_ident!("{}", name.clone()));
            }
            names.push(format_ident!("{}", name));
            column_options.push(column);

            predicates.push(syn::parse_quote!(#ty: ::sqlx::decode::Decode<#lifetime, R::Database>));
            predicates.push(syn::parse_quote!(#ty: ::sqlx::types::Type<R::Database>));

            let id = field.ident.as_ref();
            if field.default.is_some() {
                reads.push(
                    syn::parse_quote!(let #id: #ty = row.try_get(#column_name).or_else(|e| match e {
                    ::sqlx::Error::ColumnNotFound(_) => {
                        ::std::result::Result::Ok(#default)
                    },
                    e => ::std::result::Result::Err(e)
                })?;),
                );
            } else {
                reads.push(syn::parse_quote!(let #id: #ty = row.try_get(#column_name)?;));
            }
        }
    } else {
        unreachable!()
    }

    let table_def = quote! {
        #namespace::Table {
            typ: #namespace::TableType::Table(::std::borrow::Cow::Borrowed(#tablename)),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        }
    };

    // let orig_generics = &entity_def.generics;
    tokens.extend(quote! {
        impl #ident {
            const _table: #namespace::Table<'static> = #table_def;

            #(pub const #names: #namespace::ColumnOptions<#types> = #column_options;) *
        }

        impl #namespace::Entity for #ident {
            const COLUMNS: &'static [ #namespace::Column<'static> ] = &[ #(( #ident::#names.column() )), * ];

            #[inline]
            fn tablename() -> &'static str {
                #tablename
            }

            #[inline]
            fn columns() -> &'static [#namespace::Column<'static>] {
                Self::COLUMNS
            }

            #[inline]
            fn table() -> #namespace::Table<'static> {
                #ident::_table
            }
        }

    });

    if primary_key_type.is_some() {
        // impl HasPrimaryKey if PrimaryKey exists.
        let token = quote! {
            impl #ident {
                const _primary_key: <Self as #namespace::HasPrimaryKey>::PrimaryKey = #primary_key_column;
            }

            impl #namespace::HasPrimaryKey for #ident {
                type PrimaryKey = #primary_key_type;
                type PrimaryKeyValueType = #primary_key_value_type;
                #[inline]
                fn primary_key() -> <Self as #namespace::HasPrimaryKey>::PrimaryKey {
                    #ident::_primary_key
                }

                #[inline]
                fn pk(&self) -> <Self as HasPrimaryKey>::PrimaryKeyValueType {
                    self.#primary_key_column_name
                }

                #[inline]
                fn get<DB: ::sqlx::Database>(pk: Self::PrimaryKeyValueType) -> #namespace::FetchRequest<Self, DB>
                    where
                        Self: for<'r> sqlx::FromRow<'r, <DB as sqlx::Database>::Row>,
                {
                    #namespace::Select::from_table(Self::table())
                        .so_that(Self::primary_key().equals(pk))
                        .into()
                }

                #[inline]
                fn delete<'e, DB>(&'e mut self) -> #namespace::DeleteRequest<'e, Self, DB>
                    where
                        DB: ::sqlx::Database
                {
                    #namespace::DeleteRequest::new(#namespace::Delete::from_table(Self::table()).so_that(Self::primary_key().equals(self.pk())), self)
                }
            }
        };
        tokens.extend(token);
    };

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let token = quote! {
        #[automatically_derived]
        impl #impl_generics ::sqlx::FromRow<#lifetime, R> for #ident #ty_generics #where_clause {
            fn from_row(row: &#lifetime R) -> ::sqlx::Result<Self> {
                #(#reads)*

                ::std::result::Result::Ok(#ident {
                    #(#names),*
                })
            }
        }
    };

    tokens.extend(token);

    tokens.into()
}
