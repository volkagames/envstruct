#![allow(dead_code)]

use envstruct::prelude::*;
use serde::Deserialize;

#[derive(EnvStruct, Debug, PartialEq, Deserialize)]
pub struct DB {
    pub dsn: String,
    pub secret: String,
}

#[test]
fn test_from_vars_primitives() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub port: u16,
        pub hosts: Vec<String>,
        #[env(default = "info")]
        pub log_level: String,
    }

    let config = Config::from_vars([("PORT", "8080"), ("HOSTS", "a, b")]).unwrap();
    assert_eq!(config.port, 8080);
    assert_eq!(config.hosts, vec!["a", "b"]);
    assert_eq!(config.log_level, "info");

    let config =
        Config::with_prefix_from_vars("APP", [("APP_PORT", "1"), ("APP_HOSTS", "c")]).unwrap();
    assert_eq!(config.port, 1);
    assert_eq!(config.hosts, vec!["c"]);

    let res = Config::from_vars([("PORT", "nan"), ("HOSTS", "a")]);
    assert!(matches!(
        res.err().unwrap(),
        EnvStructError::ParseEnvError { var_name, .. } if var_name == "PORT"
    ));
}

#[test]
fn test_from_vars_ignores_process_env() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub path: String,
    }

    // `PATH` is set in any process that runs the tests
    assert!(std::env::var_os("PATH").is_some());
    let res = Config::from_vars(Vec::<(String, String)>::new());
    assert!(matches!(
        res.err().unwrap(),
        EnvStructError::MissingEnvVar(var_name) if var_name == "PATH"
    ));
}

#[test]
fn test_from_vars_nested() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub db: DB,
        pub replica: Option<DB>,
        pub timeout: Option<u32>,
    }

    let config = Config::with_prefix_from_vars(
        "APP",
        [("APP_DB_DSN", "dsn://main"), ("APP_DB_SECRET", "secret")],
    )
    .unwrap();
    assert_eq!(config.db.dsn, "dsn://main");
    assert_eq!(config.replica, None);
    assert_eq!(config.timeout, None);

    let config = Config::with_prefix_from_vars(
        "APP",
        [
            ("APP_DB_DSN", "dsn://main"),
            ("APP_DB_SECRET", "secret"),
            ("APP_REPLICA_DSN", "dsn://replica"),
            ("APP_REPLICA_SECRET", "secret"),
            ("APP_TIMEOUT", "30"),
        ],
    )
    .unwrap();
    assert_eq!(config.replica.unwrap().dsn, "dsn://replica");
    assert_eq!(config.timeout, Some(30));

    // any variable of an optional struct makes all of its fields required
    let res = Config::with_prefix_from_vars(
        "APP",
        [
            ("APP_DB_DSN", "dsn://main"),
            ("APP_DB_SECRET", "secret"),
            ("APP_REPLICA_DSN", "dsn://replica"),
        ],
    );
    assert!(matches!(
        res.err().unwrap(),
        EnvStructError::MissingEnvVar(var_name) if var_name == "APP_REPLICA_SECRET"
    ));
}

#[test]
fn test_from_vars_env_map() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub limits: EnvMap<String, u32>,
    }

    let config = Config::with_prefix_from_vars(
        "APP",
        [("APP_LIMITS_A", "1"), ("APP_LIMITS_B", "2"), ("OTHER", "3")],
    )
    .unwrap();
    assert_eq!(config.limits.len(), 2);
    assert_eq!(config.limits["A"], 1);
    assert_eq!(config.limits["B"], 2);
}

#[test]
fn test_from_vars_with() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(with = WithJson::<DB>)]
        pub db: DB,

        #[env(with = Doubled)]
        pub count: u32,
    }

    pub struct Doubled;
    impl Doubled {
        fn parse_from_env_vars(
            vars: &EnvVars,
            var_name: impl AsRef<str>,
            default: Option<&str>,
        ) -> Result<u32, EnvStructError> {
            Ok(u32::parse_from_env_vars(vars, var_name, default)? * 2)
        }

        fn get_env_entries(
            prefix: impl AsRef<str>,
            default: Option<&str>,
        ) -> Result<Vec<EnvEntry>, EnvStructError> {
            u32::get_env_entries(prefix, default)
        }
    }

    let config = Config::from_vars([
        ("DB", r#"{"dsn": "localhost", "secret": "my secret"}"#),
        ("COUNT", "21"),
    ])
    .unwrap();
    assert_eq!(config.db.dsn, "localhost");
    assert_eq!(config.count, 42);
}

#[test]
fn test_parse_from_env_vars_reuse() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub port: u16,
    }

    let vars = EnvVars::from([("A_PORT", "1"), ("B_PORT", "2")]);
    let a = Config::parse_from_env_vars(&vars, "A", None).unwrap();
    let b = Config::parse_from_env_vars(&vars, "B", None).unwrap();
    assert_eq!((a.port, b.port), (1, 2));
}

#[cfg(unix)]
#[test]
fn test_from_vars_non_unicode_value() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub name: String,
    }

    let res = Config::from_vars([(OsString::from("NAME"), OsString::from_vec(vec![0xff]))]);
    assert!(matches!(
        res.err().unwrap(),
        EnvStructError::InvalidVarFormat(var_name) if var_name == "NAME"
    ));
}
