use cfg_aliases::cfg_aliases;

fn main() {
    println!("cargo:rustc-cfg=docsrs");
    // Setup cfg aliases
    cfg_aliases! {
        only_mysql: { all(
            feature = "mysql",
            not(any(feature = "mssql", feature = "sqlite", feature = "postgres"))
        ) },
        only_postgres: { all(
            feature = "postgres",
            not(any(feature = "mysql", feature = "mssql", feature = "sqlite"))
        ) },
        only_sqlite: { all(
            feature = "sqlite",
            not(any(feature = "mysql", feature = "mssql", feature = "postgres"))
        ) },
        only_mssql: { all(
            feature = "mssql",
            not(any(feature = "mysql", feature = "sqlite", feature = "postgres"))
        ) },
        not_mssql: { all(
            any(feature = "mysql", feature = "sqlite", feature = "postgres"),
            not(feature = "mssql")
        ) },
        mysql_or_sqlite: { all(
            any(feature = "mysql", feature = "sqlite"),
            not(any(feature = "mssql", feature = "postgres"))
        ) },
        json: { all(
            feature = "json",
            any(feature = "postgres", feature = "mysql", feature = "sqlite"),
            not(feature = "mssql")
        ) },
        uuid: { all(
            feature = "uuid",
            any(feature = "mysql", feature = "sqlite", feature = "postgres"),
            not(feature = "mssql")
        ) },
        bigdecimal: { all(
            feature = "bigdecimal",
            any(feature = "mysql", feature = "postgres"),
            not(any(feature = "sqlite", feature = "mssql"))
        ) },
        decimal: { all(
            feature = "decimal",
            any(feature = "mysql", feature = "postgres"),
            not(any(feature = "sqlite", feature = "mssql"))
        ) },
        chrono: { all(
            feature = "chrono",
            any(feature = "mysql", feature = "sqlite", feature = "postgres"),
            not(feature = "mssql")
        ) },
        time: { all(
            feature = "time",
            not(any(feature = "mssql", feature = "sqlite"))
        ) },
        ipnetwork: { all(
            feature = "ipnetwork",
            only_postgres
        ) },
    }

    if cfg!(all(feature = "json", not(json), not(docsrs))) {
        println!("cargo:warning=feature `json` still disabled, because it only support for `MySQL` `Sqlite` and `PostgreSQL`, but `MSSQL` is enabled .");
    }

    if cfg!(all(feature = "uuid", not(uuid), not(docsrs))) {
        println!("cargo:warning=feature `uuid` still disabled, because it only support for `MySQL` `Sqlite` and `PostgreSQL`, but `MSSQL` is enabled .");
    }

    if cfg!(all(feature = "bigdecimal", not(bigdecimal), not(docsrs))) {
        println!("cargo:warning=feature `bigdecimal` still disabled, because it only support for `MySQL` and `PostgreSQL`, but `{}` is enabled .",
            if cfg!(feature = "mssql") {
                "MSSQL"
            } else if cfg!(feature = "sqlite") {
                "Sqlite"
            } else {
                unreachable!()
            }
        );
    }

    if cfg!(all(feature = "decimal", not(decimal), not(docsrs))) {
        println!("cargo:warning=feature `decimal` still disabled, because it only support for `MySQL` and `PostgreSQL`, but `{}` is enabled .",
            if cfg!(feature = "mssql") {
                "MSSQL"
            } else if cfg!(feature = "sqlite") {
                "Sqlite"
            } else {
                unreachable!()
            }
        );
    }

    if cfg!(all(feature = "chrono", not(chrono), not(docsrs))) {
        println!("cargo:warning=feature `chrono` still disabled, because it only support for `MySQL` `Sqlite` and `PostgreSQL`, but `MSSQL` is enabled .");
    }

    /*
    if cfg!(all(feature = "time", not(time))) {
        println!("cargo:warning=feature `chrono` still disabled, because it only support for `MySQL` `Sqlite` and `PostgreSQL`, but `MSSQL` is enabled .");
    }
    */
    if cfg!(all(feature = "ipnetwork", not(ipnetwork), not(docsrs))) {
        println!("cargo:warning=feature `ipnetwork` still disabled, because it only support for `PostgreSQL`, but `MySQL` `Sqlite` or `MSSQL` is enabled .");
    }
}
