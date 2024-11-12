#![allow(dead_code)]
use envstruct::prelude::*;
use serial_test::*;
use std::env;

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Config {
    pub value1: String,

    #[env(skip)]
    pub value2: String,

    #[env(skip)]
    pub value3: Option<String>,

    pub value4: i32,

    #[env(skip)]
    pub value5: std::marker::PhantomData<Foo>,
}

pub struct Foo {}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

#[test]
#[serial]
fn test_env_skip() {
    clean_env();
    env::set_var("TEST_VALUE1", "value1");
    env::set_var("TEST_VALUE2", "value2");
    env::set_var("TEST_VALUE4", "42");

    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.value1, "value1");
    assert_eq!(config.value2, "");
    assert_eq!(config.value3, None);
    assert_eq!(config.value4, 42);

    let usage = Config::usage_with_prefix("TEST").unwrap();
    println!("usage: \n{usage}");
    assert_eq!(
        usage,
        " NAME        | TYPE   | DEFAULT \n-------------+--------+---------\n TEST_VALUE1 | String |  \n TEST_VALUE4 | i32    |  \n"
    );
}
