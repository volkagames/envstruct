#![allow(dead_code)]

use envstruct::{prelude::*, BoxError};
use serde::Deserialize;
use serial_test::*;
use std::{collections::BTreeMap, env, path::PathBuf, str::FromStr};

#[allow(non_camel_case_types)]
#[derive(EnvStruct, Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum RunMode {
    local,
    remote,
    mock,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct DB {
    pub dsn: String,
    pub secret: String,
}

#[derive(Debug, PartialEq)]
pub struct Point {
    x: i32,
    y: i32,
}

impl EnvParsePrimitive for Point {
    fn parse(s: &str) -> Result<Self, BoxError> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();
        if coords.len() != 2 {
            return Err("Invalid point format".into());
        }

        let x = coords[0].parse::<i32>()?;
        let y = coords[1].parse::<i32>()?;

        Ok(Point { x, y })
    }
}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

#[test]
#[serial]
fn test_enum_parsing() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(name = "MODE")]
        pub mode: RunMode,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_MODE", "local");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.mode, RunMode::local);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_MODE", "foo");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::ParseEnvError { .. }
        ));
    }

    // undefined value
    {
        clean_env();
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::MissingEnvVar { .. }
        ));
    }
}

#[test]
#[serial]
fn test_custom_parsing() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(name = "POINT")]
        pub point: Point,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_POINT", "(1,2)");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.point, Point { x: 1, y: 2 });
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_POINT", "1,2,3");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::ParseEnvError { .. }
        ));
    }

    // undefined value
    {
        clean_env();
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::MissingEnvVar { .. }
        ));
    }
}

#[test]
#[serial]
fn test_nested_config_parsing() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub foo: Bar,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Bar {
        pub bar: Baz,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Baz {
        pub baz: String,
        pub opt: Option<String>,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_FOO_BAR_BAZ", "some value");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.foo.bar.baz, "some value");
    }

    // valid value with option
    {
        clean_env();
        env::set_var("TEST_FOO_BAR_BAZ", "some value");
        env::set_var("TEST_FOO_BAR_OPT", "other value");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.foo.bar.baz, "some value");
        assert_eq!(config.foo.bar.opt.unwrap(), "other value");
    }

    // empty value
    {
        clean_env();
        env::set_var("TEST_FOO_BAR_BAZ", "");
        env::set_var("TEST_FOO_BAR_OPT", "");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.foo.bar.baz, "");
        assert_eq!(config.foo.bar.opt.unwrap(), "");
    }

    // undefined value
    {
        clean_env();
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::MissingEnvVar { .. }
        ));
    }
}

#[test]
#[serial]
fn test_path_values() {
    use std::os::unix::ffi::OsStrExt;

    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub file_path: std::path::PathBuf,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_FILE_PATH", "/foo/bar/path");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.file_path, std::path::Path::new("/foo/bar/path"));
    }

    // invalid value
    {
        clean_env();
        env::set_var(
            "TEST_FILE_PATH",
            std::ffi::OsStr::from_bytes(b"Hello\xFFworld"),
        );
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::InvalidVarFormat { .. }
        ));
    }
}

#[test]
#[serial]
fn test_duration_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub duration: humantime::Duration,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_DURATION", "60s");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.duration.as_secs(), 60);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_DURATION", "-2s");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
    }
}

#[test]
#[serial]
fn test_bytesize_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub bytesize: bytesize::ByteSize,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_BYTESIZE", "1Mb");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.bytesize.as_u64(), 1000 * 1000);

        clean_env();
        env::set_var("TEST_BYTESIZE", "1Mib");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.bytesize.as_u64(), 1024 * 1024);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_BYTESIZE", "-42");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
    }
}

#[test]
#[serial]
fn test_url_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub url: url::Url,
    }

    // valid value
    {
        clean_env();
        env::set_var(
            "TEST_URL",
            "https://user:password@example.com/path?query=arg#hash",
        );
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.url.path(), "/path");
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_URL", "--://");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
    }
}

#[test]
#[serial]
fn test_regex_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub regex: regex::Regex,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_REGEX", "^(\\w+)-(\\w+)$");
        let config = Config::with_prefix("TEST").unwrap();
        assert!(config.regex.is_match("foo-bar"));
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_REGEX", "(\\d+");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
    }
}

#[test]
#[serial]
fn test_date_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub datetime: chrono::DateTime<chrono::FixedOffset>,
        pub utc: chrono::DateTime<chrono::Utc>,
        pub naive: chrono::NaiveDateTime,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_DATETIME", "2024-08-19T12:34:56+03:00");
        env::set_var("TEST_UTC", "2024-08-19T12:34:56+03:00");
        env::set_var("TEST_NAIVE", "2024-08-19 12:34:56");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.datetime.timestamp(), 1724060096);
        assert_eq!(config.utc.timestamp(), 1724060096);
        assert_eq!(config.naive.and_utc().timestamp(), 1724070896);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_DATETIME", "42");
        env::set_var("TEST_UTC", "42");
        env::set_var("TEST_NAIVE", "42");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
    }
}

#[test]
#[serial]
fn test_serde_values() {
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct Foo {
        pub bar: String,
        pub baz: i32,
    }

    #[derive(EnvStruct)]
    pub struct Config {
        pub value1: serde_json::Value,
        pub value2: EnvJson<Foo>,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_VALUE1", r#"{"foo": "bar"}"#);
        env::set_var("TEST_VALUE2", r#"{"bar": "example", "baz": 42}"#);
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.value1, serde_json::json!({ "foo": "bar"}));
        assert_eq!(config.value2.bar, "example");
        assert_eq!(config.value2.baz, 42);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_VALUE1", "}");
        let res = Config::with_prefix("TEST");
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(matches!(
            err,
            envstruct::EnvStructError::ParseEnvError { .. }
        ));
    }
}

#[test]
#[serial]
fn test_map_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub env_map: EnvMap<i32, String>,
    }

    clean_env();
    env::set_var("TEST_ENV_MAP_1", "foo");
    env::set_var("TEST_ENV_MAP_2", "bar");
    env::set_var("TEST_ENV_MAP_3", "baz");

    let config = Config::with_prefix("TEST").unwrap();
    println!("config {config:?}");

    assert_eq!(config.env_map.get(&1), Some(&"foo".to_string()));
    assert_eq!(config.env_map.get(&2), Some(&"bar".to_string()));
    assert_eq!(config.env_map.get(&3), Some(&"baz".to_string()));
}

#[test]
#[serial]
fn test_container_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub vec_of_strings: Vec<String>,
        pub vec_of_ints: Vec<i64>,
        pub vec_of_floats: Vec<f64>,
        pub vec_of_durations: Vec<humantime::Duration>,
        pub vec_of_vec: Vec<Vec<String>>,

        pub map_str_str: BTreeMap<String, String>,
        pub map_str_int: BTreeMap<String, i64>,
        pub map_str_vec_of_bool: BTreeMap<String, Vec<bool>>,
    }

    clean_env();
    env::set_var("TEST_VEC_OF_STRINGS", "foo,bar,baz");
    env::set_var("TEST_VEC_OF_INTS", "1, 2, 3, 4,");
    env::set_var("TEST_VEC_OF_FLOATS", "1.2, 3.4, 5.6");
    env::set_var("TEST_VEC_OF_DURATIONS", "10s, 20s, 30s");
    env::set_var("TEST_VEC_OF_VEC", "a,b,c");
    env::set_var("TEST_MAP_STR_STR", "a=b;c=d;e=f;");
    env::set_var("TEST_MAP_STR_INT", "a=1;b=2;c=3");
    env::set_var("TEST_MAP_STR_VEC_OF_BOOL", "a=true,false;b=true,true");
    let config = Config::with_prefix("TEST").unwrap();
    println!("config {config:?}");

    assert_eq!(config.vec_of_strings, vec!["foo", "bar", "baz"]);
    assert_eq!(config.vec_of_ints, vec![1, 2, 3, 4]);
    assert_eq!(config.vec_of_floats, vec![1.2, 3.4, 5.6]);
    assert_eq!(
        config.vec_of_durations,
        vec![
            humantime::Duration::from_str("10s").unwrap(),
            humantime::Duration::from_str("20s").unwrap(),
            humantime::Duration::from_str("30s").unwrap()
        ]
    );
    assert_eq!(config.vec_of_vec, vec![vec!["a"], vec!["b"], vec!["c"]]);
    assert_eq!(
        config.map_str_str,
        BTreeMap::from([
            ("a".to_string(), "b".to_string()),
            ("c".to_string(), "d".to_string()),
            ("e".to_string(), "f".to_string()),
        ])
    );
    assert_eq!(
        config.map_str_int,
        BTreeMap::from([
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ])
    );
    assert_eq!(
        config.map_str_vec_of_bool,
        BTreeMap::from([
            ("a".to_string(), vec![true, false]),
            ("b".to_string(), vec![true, true]),
        ])
    );
}

#[test]
#[serial]
fn test_rc_value() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub mode1: std::rc::Rc<RunMode>,

        #[env(default = "remote")]
        pub mode2: std::rc::Rc<RunMode>,
    }

    clean_env();
    env::set_var("TEST_MODE1", "remote");
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(*config.mode1, RunMode::remote);
    assert_eq!(*config.mode2, RunMode::remote);
}

#[test]
#[serial]
fn test_default_value() {
    #[allow(non_camel_case_types)]
    #[derive(
        EnvStruct, Debug, Default, Clone, PartialEq, Eq, strum::Display, strum::EnumString,
    )]
    pub enum Variant {
        #[default]
        undefined,
        one,
        two,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Config {
        // The `value1` field will use the default value provided by the `bool` which is `false`
        #[env(default)]
        pub value1: bool,

        // The `value2` field will default to `true` if not specified,
        // allowing non-string boolean values.
        #[env(default = true)]
        pub value2: bool,

        // The `value3` field will default to `69` if not specified,
        // allowing non-string integer values.
        #[env(default = 69)]
        pub value3: i32,

        // The `mode1` field will use "remote" as the default value if not specified.
        #[env(default = "remote")]
        pub mode1: RunMode,

        // The `mode2` field will default to the `RunMode::mock` variant if not specified.
        #[env(default = RunMode::mock)]
        pub mode2: RunMode,

        // The `mode3` field will use the default value provided by the `Variant` type's
        // implementation if not specified.
        #[env(default)]
        pub mode3: Variant,

        // The `mode4` field will be set via an environment variable or a default value provided
        // by the `Variant` type's implementation.
        #[env(default)]
        pub mode4: Variant,

        // The `mode5` field does not have a default value specified, so it must be provided
        // explicitly during initialization.
        pub mode5: Variant,
    }

    clean_env();
    env::set_var("TEST_MODE4", "one");
    env::set_var("TEST_MODE5", "two");
    let config = Config::with_prefix("TEST").unwrap();
    assert!(!config.value1);
    assert!(config.value2);
    assert_eq!(config.value3, 69);
    assert_eq!(config.mode1, RunMode::remote);
    assert_eq!(config.mode2, RunMode::mock);
    assert_eq!(config.mode3, Variant::undefined);
    assert_eq!(config.mode4, Variant::one);
    assert_eq!(config.mode5, Variant::two);
}

#[test]
#[serial]
fn test_flatten_enum() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(flatten)]
        #[env(name = "MODE")]
        pub mode: RunMode,
    }

    clean_env();
    env::set_var("TEST", "remote");
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.mode, RunMode::remote);
}

#[test]
#[serial]
fn test_flatten_struct() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(flatten)]
        pub db1: DB,

        #[env(flatten)]
        pub db2: Option<DB>,

        pub value1: DefaultTrue,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct DB {
        pub dsn: String,
        pub secret: String,
    }

    // This is a wrapper type `DefaultTrue` around a `bool`, with a EnvStruct attribute annotation.
    // The `#[env(default = true)]` attribute specifies that the default value of the
    // inner `bool` should be `true` when an instance of `DefaultTrue` is created without
    // an explicit value.
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct DefaultTrue(#[env(default = true)] pub bool);

    clean_env();
    env::set_var("TEST_DSN", "dsn://localhost/?");
    env::set_var("TEST_SECRET", "secret");
    let config = Config::with_prefix("TEST").unwrap();

    // The `dsn` field of `db1` is expected to be populated from the environment variable
    // `TEST_DSN`, so the assertion checks if `config.db1.dsn` equals "dsn://localhost/?".
    assert_eq!(config.db1.dsn, "dsn://localhost/?");

    // The `db2` field is an `Option`, so we first `unwrap()` it to access the inner value.
    // The `dsn` field of `db2` is also expected to be populated from the environment variable
    // `TEST_DSN`, so the assertion checks if `config.db2.unwrap().dsn` equals
    // "dsn://localhost/?".
    assert_eq!(config.db2.unwrap().dsn, "dsn://localhost/?");

    assert!(config.value1.0);
}

#[test]
#[serial]
fn test_with_override() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(with = OverrideStringEnv)]
        pub value1: String,

        #[env(with = WithJson::<DB>)]
        pub db: DB,
    }

    pub struct OverrideStringEnv;
    impl OverrideStringEnv {
        fn parse_from_env_var(
            _var_name: impl AsRef<str>,
            _default: Option<&str>,
        ) -> Result<String, EnvStructError> {
            Ok("override value".to_string())
        }

        fn get_env_entries(
            prefix: impl AsRef<str>,
            default: Option<&str>,
        ) -> Result<Vec<EnvEntry>, EnvStructError> {
            String::get_env_entries(prefix, default)
        }
    }

    #[derive(EnvStruct, Debug, PartialEq, Deserialize)]
    pub struct DB {
        pub dsn: String,
        pub secret: String,
    }

    clean_env();
    env::set_var("TEST_VALUE1", "foo");
    env::set_var("TEST_DB", r#"{"dsn": "localhost", "secret": "my secret"}"#);
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.value1, "override value");
    assert_eq!(config.db.dsn, "localhost");
}

#[test]
#[serial]
fn test_usage_output() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub file_path: PathBuf,
        pub duration: humantime::Duration,
        pub bytesize: bytesize::ByteSize,
        pub url: url::Url,
        pub regex: regex::Regex,
        pub datetime: chrono::DateTime<chrono::FixedOffset>,
        pub utc: chrono::DateTime<chrono::Utc>,
        pub naive: chrono::NaiveDateTime,

        pub mode: RunMode,
        pub db: DB,
        pub point: Point,

        pub vec_of_strings: Vec<String>,
        pub vec_of_ints: Vec<i64>,
        pub vec_of_floats: Vec<f64>,
        pub vec_of_durations: Vec<humantime::Duration>,
        pub vec_of_vec: Vec<Vec<String>>,

        pub map_str_str: BTreeMap<String, String>,
        pub map_str_int: BTreeMap<String, i64>,
        pub map_str_vec_of_bool: BTreeMap<String, Vec<bool>>,

        pub env_map: EnvMap<i32, String>,
    }

    clean_env();
    let usage = Config::usage_with_prefix("TEST").unwrap();
    println!("usage: \n{usage}");
    assert!(usage.contains("TEST_FILE_PATH"));
    assert!(usage.contains("TEST_VEC_OF_STRINGS"));
    assert!(usage.contains("TEST_MAP_STR_VEC_OF_BOOL"));
}

#[test]
#[serial]
fn test_optional_struct() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(flatten)]
        pub foo: Option<Foo>,
        pub bar: Option<Bar>,
        pub baz: Option<Baz>,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Foo {
        pub url: url::Url,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Bar {
        pub name: String,
        #[env(default = "")]
        pub other: String,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Baz {
        pub dsn: String,
        pub ttl: u64,
    }

    let usage = Config::usage_with_prefix("TEST").unwrap();
    println!("usage: \n{usage}");

    // all fields are expected to be optional, not required, and flatten
    // does not trigger any errors.
    {
        clean_env();
        let config = Config::with_prefix("TEST").unwrap();
        assert!(config.foo.is_none());
        assert!(config.bar.is_none());
    }

    // setting at least one variable of BAR to make field required
    {
        clean_env();
        env::set_var("TEST_BAR_NAME", "bar");
        let config = Config::with_prefix("TEST").unwrap();
        assert!(config.foo.is_none());
        assert!(config.bar.is_some());
    }

    // setting TTL makes DSN a required field. Since DSN is not defined, this causes an error.
    {
        clean_env();
        env::set_var("TEST_BAZ_TTL", "42");
        let res = Config::with_prefix("TEST");
        println!("res: {res:?}");
        assert!(res.is_err());
        assert!(matches!(
            res.err().unwrap(),
            envstruct::EnvStructError::MissingEnvVar { .. }
        ));
    }
}
