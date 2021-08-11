#[macro_use]
extern crate darling;

use darling::{FromDeriveInput, FromMeta, ToTokens};
use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

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

macro_rules! quote_option {
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

//uses_type_params!(EntityDefinition, ty);
//uses_type_params!(ColumnOptions, ty);

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
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut column_options = Vec::new();
    if let darling::ast::Data::Struct(darling::ast::Fields { fields, .. }) = entity_def.data {
        for field in fields.into_iter() {
            // println!("field: {:?}", field);
            let ty = field.ty;
            types.push(ty.clone());
            let default_name = field.ident.map(|i| i.to_string()).unwrap();
            let name = field.name.unwrap_or(default_name);
            let is_primary_key = field.primary_key.is_some();
            let autoincrement = field.autoincrement.is_some();
            let comment = quote_option!(field.comment.map(|v| { v.value().to_string() }));
            let foreign_key = quote_option!(field.foreign_key.map(|v| v.value().to_string()));
            let unique = field.unique;
            let length = quote_option!(field.length);
            let quote_name = field.quote;
            let default = quote_option!(field.default);
            let onupdate = quote_option!(field.onupdate);
            let server_default = quote_option!(field.server_default);
            let server_onupdate = quote_option!(field.server_onupdate);
            let column = quote! {
                ::xiayu::prelude::ColumnOptions::new(
                    /* name: */ #name,
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
                primary_key_type = Some(quote! { #ty });
                // println!("primary_key_definition: {:?}", column.clone().to_string());
                primary_key_column = Some(column.clone());
            }
            names.push(format_ident!("{}", name));
            column_options.push(column);
        }
    } else {
        unreachable!()
    }
    let primary_key_type = match primary_key_type {
        Some(pk) => pk,
        None => {
            quote_spanned! { input.span() => ::std::compile_error!( "`PrimaryKey` is missing." ) }
        }
    };
    let mut tokens = TokenStream2::new();

    // let stringified_names = names.iter().map(|name| name.to_string());

    tokens.extend(quote! {
        impl #ident {
            pub const tablename: &'static str = #tablename;

            pub const primary_key: ::xiayu::prelude::ColumnOptions<#primary_key_type> = #primary_key_column;

            #(pub const #names: ::xiayu::prelude::ColumnOptions<#types> = #column_options;) *
        }

        impl ::xiayu::prelude::Entity for #ident {
            type PrimaryKey = ::xiayu::prelude::ColumnOptions<#primary_key_type>;

            const COLUMNS: &'static [ ::xiayu::prelude::Column<'static> ] = &[ #(( #ident::#names.column() )), * ];

            #[inline]
            fn primary_key() -> <Self as ::xiayu::prelude::Entity>::PrimaryKey { 
                #ident::primary_key
            }

            #[inline]
            fn tablename() -> &'static str {
                #tablename
            }

            #[inline]
            fn columns() -> &'static [::xiayu::prelude::Column<'static>] {
                // &[ #(( #ident::#names.column() )), * ]
                Self::COLUMNS
            }

            #[inline]
            fn table() -> ::xiayu::prelude::Table<'static> {
                #ident::primary_key.table()
            }
        }
    });
    tokens.into()
}
